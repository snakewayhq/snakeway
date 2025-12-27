fn main() {
    tonic_prost_build::compile_protos("proto/helloworld.proto").expect("failed to compile protos");
}
