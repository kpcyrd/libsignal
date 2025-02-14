//
// Copyright (C) 2024 Signal Messenger, LLC.
// SPDX-License-Identifier: AGPL-3.0-only
//

use std::path::PathBuf;

use assert_cmd::Command;
use assert_matches::assert_matches;
use dir_test::{dir_test, Fixture};
use futures::io::Cursor;
use futures::AsyncRead;
use libsignal_message_backup::frame::FileReaderFactory;
use libsignal_message_backup::key::{BackupKey, MessageBackupKey};
use libsignal_message_backup::{BackupReader, ReadResult};
use libsignal_protocol::Aci;

#[dir_test(
        dir: "$CARGO_MANIFEST_DIR/tests/res/test-cases",
        glob: "valid/*.jsonproto",
        postfix: "binproto"
    )]
fn is_valid_json_proto(input: Fixture<&str>) {
    let json_contents = input.into_content();
    let json_contents = serde_json::from_str(json_contents).expect("invalid JSON");
    let json_array = assert_matches!(json_contents, serde_json::Value::Array(contents) => contents);
    let binary =
        libsignal_message_backup::backup::convert_from_json(json_array).expect("failed to convert");

    // Check via the library interface.
    let input = Cursor::new(&*binary);
    let reader = BackupReader::new_unencrypted(input);
    validate(reader);

    // The CLI tool should agree.
    validator_command()
        .arg("-")
        .write_stdin(binary)
        .ok()
        .expect("command failed");
}

#[dir_test(
        dir: "$CARGO_MANIFEST_DIR/tests/res/test-cases",
        glob: "valid/*.binproto.encrypted",
        loader: PathBuf::from,
        postfix: "encrypted"
    )]
fn is_valid_encrypted_proto(input: Fixture<PathBuf>) {
    const ACI: Aci = Aci::from_uuid_bytes([0x11; 16]);
    const MASTER_KEY: [u8; 32] = [b'M'; 32];
    let backup_key = BackupKey::derive_from_master_key(&MASTER_KEY);
    let key = MessageBackupKey::derive(&backup_key, &backup_key.derive_backup_id(&ACI));

    let path = input.into_content();
    // Check via the library interface.
    let factory = FileReaderFactory { path: &path };
    let reader = futures::executor::block_on(BackupReader::new_encrypted_compressed(&key, factory))
        .expect("invalid HMAC");
    validate(reader);

    // The CLI tool should agree.
    validator_command()
        .args([
            "--aci".to_string(),
            ACI.service_id_string(),
            "--master-key".to_string(),
            hex::encode(MASTER_KEY),
            path.to_string_lossy().into_owned(),
        ])
        .ok()
        .expect("command failed");
}

const EXPECTED_SUFFIX: &str = "jsonproto.expected";
#[dir_test(
    dir: "$CARGO_MANIFEST_DIR/tests/res/test-cases",
    glob: "invalid/*.jsonproto",
    loader: PathBuf::from
)]
fn invalid_jsonproto(input: Fixture<PathBuf>) {
    let path = input.into_content();
    let expected_path = path.with_extension(EXPECTED_SUFFIX);

    let json_contents =
        serde_json::from_str(&std::fs::read_to_string(path).expect("failed to read"))
            .expect("invalid JSON");
    let json_array = assert_matches!(json_contents, serde_json::Value::Array(contents) => contents);
    let binary =
        libsignal_message_backup::backup::convert_from_json(json_array).expect("failed to convert");

    let input = Cursor::new(&*binary);
    let reader = BackupReader::new_unencrypted(input);

    let ReadResult {
        result,
        found_unknown_fields: _,
    } = futures::executor::block_on(reader.read_all());

    let text = result.expect_err("unexpectedly valid").to_string();

    if write_expected_error() {
        eprintln!("writing expected value to {:?}", expected_path);
        std::fs::write(expected_path, text).expect("failed to overwrite expected contents");
        return;
    }

    let expected_text =
        std::fs::read_to_string(&expected_path).expect("can't load expected contents");

    assert_eq!(text, expected_text);
}

fn write_expected_error() -> bool {
    std::env::var_os("OVERWRITE_EXPECTED_OUTPUT").is_some()
}

fn validate(mut reader: BackupReader<impl AsyncRead + Unpin>) {
    reader.visitor = |msg| println!("{msg:#?}");

    let ReadResult {
        result,
        found_unknown_fields,
    } = futures::executor::block_on(reader.read_all());
    assert_eq!(found_unknown_fields, Vec::new());

    let backup = result.expect("invalid backup");
    println!("got backup:\n{backup:#?}");
}

fn validator_command() -> Command {
    Command::cargo_bin("validator").expect("bin not found")
}
