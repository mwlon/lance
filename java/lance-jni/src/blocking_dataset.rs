// Copyright 2023 Lance Developers.
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

use arrow::array::RecordBatchReader;

use jni::objects::JObject;
use jni::JNIEnv;
use lance::dataset::{Dataset, WriteParams};

use crate::{traits::IntoJava, Result, RT};

pub struct BlockingDataset {
    inner: Dataset,
}

impl BlockingDataset {
    pub fn write(
        reader: impl RecordBatchReader + Send + 'static,
        uri: &str,
        params: Option<WriteParams>,
    ) -> Result<Self> {
        let inner = RT.block_on(Dataset::write(reader, uri, params))?;
        Ok(Self { inner })
    }

    pub fn open(uri: &str) -> Result<Self> {
        let inner = RT.block_on(Dataset::open(uri))?;
        Ok(Self { inner })
    }

    pub fn count_rows(&self) -> Result<usize> {
        Ok(RT.block_on(self.inner.count_rows())?)
    }

    pub fn close(&self) {}
}

impl IntoJava for BlockingDataset {
    fn into_java<'a>(self, env: &mut JNIEnv<'a>) -> JObject<'a> {
        attach_native_dataset(env, self)
    }
}

fn attach_native_dataset<'local>(
    env: &mut JNIEnv<'local>,
    dataset: BlockingDataset,
) -> JObject<'local> {
    let j_dataset = create_java_dataset_object(env);
    // This block sets a native Rust object (dataset) as a field in the Java object (j_dataset).
    // Caution: This creates a potential for memory leaks. The Rust object (dataset) is not
    // automatically garbage-collected by Java, and its memory will not be freed unless
    // explicitly handled.
    //
    // To prevent memory leaks, ensure the following:
    // 1. The Java object (`j_dataset`) should implement the `java.io.Closeable` interface.
    // 2. Users of this Java object should be instructed to always use it within a try-with-resources
    //    statement (or manually call the `close()` method) to ensure that `self.close()` is invoked.
    match unsafe { env.set_rust_field(&j_dataset, "nativeDatasetHandle", dataset) } {
        Ok(_) => j_dataset,
        Err(err) => {
            env.throw_new(
                "java/lang/RuntimeException",
                format!("Failed to set native handle: {}", err),
            )
            .expect("Error throwing exception");
            JObject::null()
        }
    }
}

fn create_java_dataset_object<'a>(env: &mut JNIEnv<'a>) -> JObject<'a> {
    env.new_object("com/lancedb/lance/Dataset", "()V", &[])
        .expect("Failed to create Java Dataset instance")
}
