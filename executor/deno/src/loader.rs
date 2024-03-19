use anyhow::{anyhow, Context, Result};
use deno_core::{url::Url, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier};
use reqwest::{Client, ClientBuilder};
use std::{pin::Pin, rc::Rc};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone)]
pub struct RegisterLoader {
    client: Client,
}

impl RegisterLoader {
    pub fn new() -> RegisterLoader {
        let client = ClientBuilder::new()
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap();
        RegisterLoader { client }
    }
}

impl ModuleLoader for RegisterLoader {
    fn resolve(&self, specifier: &str, referrer: &str, is_main: bool) -> Result<ModuleSpecifier> {
        if !is_main {
            return Err(anyhow!("import module is not allowed for now!"));
        }
        let url = Url::parse(specifier)?;
        if referrer != "." {
            tracing::warn!(referrer, "Ignored referrer");
        }
        if url.cannot_be_a_base() {
            return Err(anyhow!("Invalid URL: {}", specifier));
        }
        if !matches!(url.scheme(), "http" | "https") {
            return Err(anyhow!("Invalid scheme: {}", url.scheme()));
        }
        Ok(url)
    }

    // TODO: check the host
    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<ModuleSpecifier>,
        is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        if is_dyn_import {
            return Box::pin(async { Err(anyhow!("dynamic load module is not allowed for now!")) });
        }
        if let Some(referrer) = maybe_referrer {
            tracing::error!("Ignored referrer {}", referrer);
        }
        let module_url_specified = module_specifier.as_str().to_owned();
        let req = self.client.get(module_specifier.clone());
        Box::pin(async {
            let res = req.send().await?;
            if res.status().is_success() {
                let module_url_found = res.url().as_str().to_owned();
                let code = res.text().await?.into_bytes().into_boxed_slice();
                Ok(ModuleSource {
                    code,
                    module_url_specified,
                    module_url_found,
                    module_type: deno_core::ModuleType::JavaScript,
                })
            } else {
                Err(anyhow!(
                    "Get module {} failed with status: {}",
                    module_url_specified,
                    res.status()
                ))
            }
        })
    }
}

#[derive(Clone)]
pub struct FsLoader;

impl ModuleLoader for FsLoader {
    fn resolve(&self, specifier: &str, referrer: &str, is_main: bool) -> Result<ModuleSpecifier> {
        if !is_main {
            return Err(anyhow!("import module is not allowed for now!"));
        }
        let url = Url::parse(specifier)?;
        tracing::warn!(referrer, "Ignored referrer");
        Ok(url)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<ModuleSpecifier>,
        is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        if is_dyn_import {
            return Box::pin(async { Err(anyhow!("dynamic load module is not allowed for now!")) });
        }
        if let Some(referrer) = maybe_referrer {
            tracing::warn!("Ignored referrer {}", referrer);
        }
        if let Ok(path) = module_specifier.to_file_path() {
            Box::pin(async move {
                let mut file = File::open(&path)
                    .await
                    .with_context(|| format!("{:?} Not found", &path))?;
                let mut code = Vec::new();
                file.read_to_end(&mut code).await?;
                let module_url_found = Url::from_file_path(path.canonicalize()?).unwrap();
                Ok(ModuleSource {
                    code: code.into_boxed_slice(),
                    module_url_specified: path.to_string_lossy().to_string(),
                    module_url_found: module_url_found.to_string(),
                    module_type: deno_core::ModuleType::JavaScript,
                })
            })
        } else {
            return Box::pin(async { Err(anyhow!("dynamic load module is not allowed for now!")) });
        }
    }
}
