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
    if rustc_version::version_meta().unwrap().commit_hash.unwrap() != "03fc9d622e0ea26a3d37f5ab030737fcca6928b9" {
        println!("WARNING: expected rustc version 1.18.0 (03fc9d622 2017-06-06). (if you want to use a different one make sure the declarations and functions in dump_std correspond to your rustc's source code)")
    }
}
