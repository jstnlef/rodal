extern crate rustc_version;

fn main() {
    if rustc_version::version_meta().unwrap().commit_hash.unwrap() != "03fc9d622e0ea26a3d37f5ab030737fcca6928b9" {
        panic!("expected rustc version 1.18.0 (03fc9d622 2017-06-06). (if you want to use a different one make sure the declarations and functions in dump_std correspond to your rustc's source code)")
    }
}
