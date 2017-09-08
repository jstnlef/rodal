// Copyright 2017 The Australian National University
// 
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate rustc_version;

fn main() {
    let hash = rustc_version::version_meta().unwrap().commit_hash.unwrap();
    if  hash != "03fc9d622e0ea26a3d37f5ab030737fcca6928b9" && hash != "0ade339411587887bf01bcfa2e9ae4414c8900d4" && hash != "10d7cb44c98f25c04dcefb6b6555237de8b8bd7e" && hash != "f3d6973f41a7d1fb83029c9c0ceaf0f5d4fd7208" {
        panic!("expected rustc version 1.18.0 (03fc9d622 2017-06-06) or 1.19.0 (0ade33941 2017-07-17) or 1.19.0-nightly (10d7cb44c 2017-06-18) or 1.20.0 (f3d6973f4 2017-08-27) (if you want to use a different one make sure the declarations and functions in rust_std.rs correspond to your rustc's source code)")
    }
}
