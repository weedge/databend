// Copyright 2023 Datafuse Labs.
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

use std::sync::Arc;

use common_exception::Result;
use common_meta_kvapi::kvapi::KVApi;
use common_meta_store::MetaStore;
use common_meta_types::KVMeta;
use common_meta_types::MatchSeq;
use common_meta_types::Operation;
use common_meta_types::SeqV;
use common_meta_types::UpsertKV;

use crate::common::ResultCacheValue;

pub(super) struct ResultCacheMetaManager {
    key: String,
    ttl: u64,
    inner: Arc<MetaStore>,
}

impl ResultCacheMetaManager {
    pub fn create(inner: Arc<MetaStore>, key: String, ttl: u64) -> Self {
        Self { key, ttl, inner }
    }

    pub async fn set(&self, value: ResultCacheValue, seq: MatchSeq, expire_at: u64) -> Result<()> {
        let value = serde_json::to_vec(&value)?;
        let _ = self
            .inner
            .upsert_kv(UpsertKV {
                key: self.key.clone(),
                seq,
                value: Operation::Update(value),
                value_meta: Some(KVMeta {
                    expire_at: Some(expire_at),
                }),
            })
            .await?;
        Ok(())
    }

    pub async fn get(&self) -> Result<Option<ResultCacheValue>> {
        let raw = self.inner.get_kv(&self.key).await?;
        match raw {
            None => Ok(None),
            Some(SeqV { data, .. }) => {
                let value = serde_json::from_slice(&data)?;
                Ok(Some(value))
            }
        }
    }

    pub fn get_ttl(&self) -> u64 {
        self.ttl
    }
}
