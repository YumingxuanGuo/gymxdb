fn main() {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["src/proto/database.proto"], &["proto"])
        .unwrap();
}