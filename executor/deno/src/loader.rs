use anyhow::{anyhow, Context, Result};
use deno_core::{url::Url, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier};
use reqwest::{Client, ClientBuilder};
use std::{pin::Pin, rc::Rc};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
//。这两个加载器分别从远程URL和本地文件系统加载Deno模块
#[derive(Debug, Clone)]
pub struct RegisterLoader {
    client: Client,//负责通过HTTP(S)请求从指定的URL加载模块源代码
}

impl RegisterLoader {//构造函数，用于创建RegisterLoader的实例。
    pub fn new() -> RegisterLoader {
        let client = ClientBuilder::new()
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap();
        RegisterLoader { client }
    }
}

impl ModuleLoader for RegisterLoader {//ModuleLoader负责解析给定的模块标识符到具体的URL或路径
    //给定一个模块标识符（specifier）、引用它的模块的标识符（referrer）和一个标志表示是否为主模块（is_main）
    fn resolve(&self, specifier: &str, referrer: &str, is_main: bool) -> Result<ModuleSpecifier> {
        if !is_main {//这个方法尝试解析模块标识符到一个ModuleSpecifier实例
            return Err(anyhow!("import module is not allowed for now!"));
        }
        let url = Url::parse(specifier)?;//尝试将模块标识符解析为Url实例。
        if referrer != "." {
            tracing::warn!(referrer, "Ignored referrer");
        }
        if url.cannot_be_a_base() {//如果解析出的URL不能作为基础（cannot_be_a_base返回true），则返回错误，表明无效的
            return Err(anyhow!("Invalid URL: {}", specifier));
        }
        if !matches!(url.scheme(), "http" | "https") {
            return Err(anyhow!("Invalid scheme: {}", url.scheme()));
        }
        Ok(url)
    }//提供了一种仅允许从网络加载主模块且对URL有特定要求的加载策略

    // TODO: check the host
    fn load(
        &self,
        module_specifier: &ModuleSpecifier,//模块的标识符（URL或路径），表示要加载的模块。
        maybe_referrer: Option<ModuleSpecifier>,//可选的模块标识符，表示引用该模块的父模块
        is_dyn_import: bool,//布尔值，表示模块是否通过动态导入(import())请求加载
    ) -> Pin<Box<ModuleSourceFuture>> {
        if is_dyn_import {//表示请求是动态导入，此实现立即返回一个永远失败的异步操作，说明当前不允许动态加载模块
            return Box::pin(async { Err(anyhow!("dynamic load module is not allowed for now!")) });
        }
        if let Some(referrer) = maybe_referrer {//如果提供了引用者(maybe_referrer)，记录一条错误日志并继续。这表明实现中忽略了引用者信息。
            tracing::error!("Ignored referrer {}", referrer);
        }
        let module_url_specified = module_specifier.as_str().to_owned();
        let req = self.client.get(module_specifier.clone());//使用reqwest客户端的get方法，构建请求对象
        Box::pin(async {
            let res = req.send().await?;
            if res.status().is_success() {//如果请求成功（响应状态码为2xx），则从响应中读取模块代码，并构建ModuleSource结构体返回。
                let module_url_found = res.url().as_str().to_owned();
                let code = res.text().await?.into_bytes().into_boxed_slice();
                Ok(ModuleSource {
                    code,
                    module_url_specified,//指定的模块URL（module_url_specified）、实际找到的模块URL（module_url_found）和模块类型（这里固定为JavaScript）
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
        let url = Url::parse(specifier)?;//模块标识符
        tracing::warn!(referrer, "Ignored referrer");
        Ok(url)
    }//解析给定的模块标识符到一个ModuleSpecifier实例

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
            Box::pin(async move {//如果module_specifier能够转换为有效的文件路径，该方法会异步打开并读取该文件，将其内容作为模块代码返回。
                let mut file = File::open(&path)
                    .await
                    .with_context(|| format!("{:?} Not found", &path))?;
                let mut code = Vec::new();
                file.read_to_end(&mut code).await?;// 将文件内容读取到缓冲区
                let module_url_found = Url::from_file_path(path.canonicalize()?).unwrap();
                Ok(ModuleSource {//成功读取文件后，构建并返回一个ModuleSource实例，其中包含模块代码、指定的模块URL和实际找到的模块URL（可能因重定向而有所不同）以及模块类
                    code: code.into_boxed_slice(),
                    module_url_specified: path.to_string_lossy().to_string(),
                    module_url_found: module_url_found.to_string(),
                    module_type: deno_core::ModuleType::JavaScript,
                })
            })
        } else {
            return Box::pin(async { Err(anyhow!("dynamic load module is not allowed for now!")) });
        }
    }//FsLoader为从本地文件系统加载JavaScript模块提供了支持。这对于那些需要从特定位置加载代码模块的应用程序非常有用
}
