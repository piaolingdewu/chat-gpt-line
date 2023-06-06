pub mod g_bot {
    use reqwest::Client;
    use crate::g_bot::history::History;
    use crate::g_config::g_config;
    use crate::g_config::g_config::config;

    //请求
    #[derive(Debug)]
    pub struct g_bot {
        //查看配置文件
        pub config: config,
    }

    impl g_bot {
        pub fn new() -> Self {
            //载入配置文件
            let config = g_config::config::load_config();

            g_bot {
                config,
            }
        }

        pub async fn send_qustion(&mut self, quest: String, tx: tokio::sync::mpsc::Sender<String>) {
            //载入历史记录
            let mut history = vec![];
            if let Some(load) = History::new().read_history_content(self.config.memory) {
                history = load;
            }

            // 这里临时修改一下参数
            super::gpt_request::gpt_request::new()
                .add_http_proxy(self.config.http_proxy.clone())
                .add_https_proxy(self.config.https_proxy.clone())
                .add_token(self.config.token.clone())
                .add_QA_histore(history)
                .set_model_name(self.config.module.clone())
                .enable_stream(self.config.stream.clone())
                .set_qustion(quest)
                .send(tx).await;
        }
    }

    #[tokio::test]
    async fn bot_test() {
        //发送简单的请求
        let mut bot = g_bot::new();

        println!("{:?}", bot.config);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

        tokio::spawn(async move {
            bot.send_qustion("帮我使用rust编写一个hello world程序".to_string(), tx).await;
        });

        while let Some(chunk) = rx.recv().await {
            print!("{}", chunk);
        }
    }

    #[tokio::test]
    async fn row_request() {
        // 新建一个post请求
        let mut request = Client::new()
            .post("https://accesschatgpt.openai.azure.com/openai/deployments/gpt/chat/completions?api-version=2023-03-15-preview")
            .header("Content-Type", "application/json")
            .header("api-key", "<KEY>")
            .body(r#"{
                      "messages": [
                          {"role":"system","content":"You are an AI assistant that helps people find information."},
                          {"role":"user","content":"帮我使用python编写一个hello world 程序?"}
                          ],
                      "max_tokens": 2000,
                      "temperature": 0.7,
                      "frequency_penalty": 0,
                      "presence_penalty": 0,
                      "top_p": 0.95,
                      "stop": null
                    }"#)
            .build()
            .unwrap();

        // 发送请求
        let res = Client::new().execute(request).await.unwrap();

        println!("{:?}", res.text().await.unwrap());
    }
}

//gpt的请求
mod gpt_request {
    use std::fmt::Debug;
    use reqwest::{Client, RequestBuilder};
    use serde::Serialize;
    use telegram_bot::Message;
    use crate::g_bot::history::QA;

    use crate::g_bot::request_json::*;
    use crate::g_bot::request_json::recv_chunk::ChatCompletionChunk;

    #[derive(Default)]
    pub struct gpt_request {
        //用于提问的token
        token: String,

        // 问题
        question: String,
        // 回答
        anwser: String,

        //代理
        http_proxy: String,
        https_proxy: String,

        //启用流推送
        stream: bool,
        //设置使用模型的名称
        model: String,
        //设置历史对话内容
        qa: Vec<super::history::QA>,
        //设置系统的人设
        system_character: String,
    }

    impl gpt_request {
        //新建一个请求
        pub fn new() -> Self {
            let mut this = Self { ..Default::default() };
            return this;
        }
        pub fn add_token(&mut self, token: String) -> &mut gpt_request {
            self.token = token;
            return self;
        }

        pub fn add_http_proxy(&mut self, proxy: String) -> &mut gpt_request {
            self.http_proxy = proxy;
            return self;
        }
        pub fn add_https_proxy(&mut self, proxy: String) -> &mut gpt_request {
            self.https_proxy = proxy;
            return self;
        }
        pub fn add_QA_histore(&mut self, qa: Vec<QA>) -> &mut gpt_request {
            self.qa.append(&mut qa.clone());
            return self;
        }
        pub fn enable_stream(&mut self, bstream: bool) -> &mut gpt_request {
            self.stream = true;
            return self;
        }
        pub fn set_model_name(&mut self, model: String) -> &mut gpt_request {
            self.model = model;
            return self;
        }
        pub fn set_qustion(&mut self, qustion: String) -> &mut gpt_request {
            self.question = qustion;
            return self;
        }

        //构造消息
        fn construct_messages(&mut self) {
            todo!()
        }

        // 发送azure的请求
        pub async fn send_azure(&mut self, sender: tokio::sync::mpsc::Sender<String>) {
            todo!()
        }
        //发送请求
        pub async fn send(&mut self, sender: tokio::sync::mpsc::Sender<String>) {
            let mut body = super::request_json::request_body::ChatMessage { ..Default::default() };

            body.model = self.model.clone();
            body.stream = self.stream.clone();

            for item in self.qa.iter() {
                body.messages.push(super::request_json::request_body::Message {
                    role: "user".to_string(),
                    content: item.qustion.to_string(),
                });
                body.messages.push(super::request_json::request_body::Message {
                    role: "assistant".to_string(),
                    content: item.anwser.to_string(),
                })
            }
            body.messages.push(super::request_json::request_body::Message {
                role: "user".to_string(),
                content: self.question.clone().to_string(),
            });


            let http_proxy = self.http_proxy.clone();
            let token = self.token.clone();
            let is_stream = self.stream.clone();

            let mut request=if http_proxy.is_empty() { Client::new() }else {
                Client::builder().proxy(reqwest::Proxy::http(&http_proxy).unwrap()).build().unwrap()
            };

            match request.post("https://accesschatgpt.openai.azure.com/openai/deployments/gpt/chat/completions?api-version=2023-03-15-preview")
                         .header("Content-Type", "application/json")
                         .header("api-key", format!("{}", token.clone()))
                         .body(serde_json::to_string(&body).unwrap())
                         .send().await {
                Ok(mut client) => {
                    // 请求成
                    //println!("{}",serde_json::to_string(&body).unwrap());

                    if is_stream {
                        while let Some(Recv) = client.chunk().await.unwrap() {
                            //解析chunk并发送
                             
                            let mut str = match String::from_utf8(Recv.to_vec()) {
                                Ok(data) => {data},
                                Err(err) => {
                                    // 解析失败 并且返回错误内容
                                    panic!("{}",err);
                                },
                            };

                            for line in str.lines() {
                                //格式化文本
                                if let Some(json_data) = line.split_once("data:") {
                                    // 这里进行splite会失败？ 因为整好卡在了两个字符的中间
                                    //let json_data = line.split_at(5).1;
                                    
                                    match serde_json::from_str::<super::request_json::recv_chunk::ChatCompletionChunk>(&json_data.1) {
                                        Ok(data) => {
                                            let out_string = data.choices[0].delta.content.clone();
                                            sender.send(out_string).await.unwrap();
                                        }
                                        Err(err) => {
                                            //println!("{}",err);
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        while let Some(Recv) = client.chunk().await.unwrap() {
                            //解析非流式传输的文件回复
                            let str = String::from_utf8(Recv.to_vec()).unwrap();

                            let rec: super::request_json::recv::ChatCompletion = serde_json::from_str(str.as_str()).unwrap();

                            sender.send(rec.choices[0].message.content.clone()).await.unwrap();
                        }
                    }
                }
                Err(err) => {
                    // 请求失败
                    panic!("{}", format!("{}", err));
                }
            }
        }
    }
}

//历史记录
pub mod history {
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use dirs;
    use nix;
    use serde::{Deserialize, Serialize};

    use serde::de::Unexpected::Option;

    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct QA {
        pub qustion: String,
        pub anwser: String,
    }

    impl QA {
        //保存上下文
        pub(crate) fn save(&self) {
            todo!()
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct QaRecord {
        ppid: String,
        qa: Vec<QA>,
    }

    #[derive(Debug)]
    pub struct History {
        //读取yaml文件
        record: QaRecord,
        file_path: PathBuf,
    }

    impl History {
        pub fn read_history_content(&self, memory: u8) -> std::option::Option<Vec<QA>> {
            let config = self.file_path.clone();
            if let Ok(content) = std::fs::read_to_string(config) {
                let mut rindex = memory;
                let record: QaRecord = serde_json::from_str(&content).unwrap();
                let mut qa_list = record.qa;
                qa_list.reverse();
                let mut out = vec![];

                for item in qa_list.iter() {
                    if rindex == 0 { break; }

                    out.push(item.clone());
                    rindex -= 1;
                }
                out.reverse();
                return std::option::Option::Some(out);
            }
            return None;
        }
        pub fn write_to_history_content(&mut self, qa: QA) {
            self.record.qa.push(qa.clone());

            match std::fs::File::open(self.file_path.clone()) {
                Ok(mut file) => {
                    // 文件已成功打开，你可以在这里对文件进行读取或写入
                    let mut file_content = String::new();
                    file.read_to_string(&mut file_content).unwrap();
                    let mut json_data: QaRecord = serde_json::from_str(&file_content).unwrap();
                    json_data.qa.push(qa.clone());
                    file.write_all("".as_bytes()).unwrap();
                    //file.write(serde_json::to_string(&json_data).unwrap().as_bytes()).unwrap();

                    std::fs::write(&self.file_path, serde_json::to_string(&json_data).unwrap().as_bytes()).unwrap()
                }
                Err(error) => {
                    // 文件打开失败的处理，来自错误error变量
                    //新建一个文件
                    let string = serde_json::to_string(&self.record).unwrap();
                    std::fs::write(&self.file_path, string).unwrap();
                }
            }
        }

        #[cfg(windows)]
        pub fn get_ppid() -> String {
            return format!("{}", 0);
        }
        #[cfg(unix)]
        pub fn get_ppid() -> String {
            return nix::unistd::getppid().to_string();
        }

        pub fn new() -> History {
            //读取文件
            let mut config = dirs::config_dir().unwrap();
            config.push("chat-gpt-line");
            config.push("history");
            if !config.exists() {
                std::fs::create_dir_all(&config).unwrap();
            }
            let ppid = History::get_ppid();

            config.push(ppid.clone() + ".json");


            if let Ok(content) = std::fs::read_to_string(config.clone()) {
                //如果文件存在
                let record: QaRecord = serde_json::from_str(&content).unwrap();
                return History {
                    record: record,
                    file_path: config.clone(),
                };
            }

            //文件不存在就创建内容并且写入文件
            let record = QaRecord {
                ppid: ppid.to_string(),
                qa: vec![],
            };

            return History {
                record,
                file_path: config.clone(),
            };
        }
    }

    #[test]
    #[cfg(unix)]
    fn file_write_test() {
        let mut qa = QaRecord {
            ppid: nix::unistd::getppid().to_string(),
            qa: vec![],
        };
        let q1 = QA {
            qustion: "你是?".to_string(),
            anwser: "你猜？".to_string(),
        };
        let q2 = QA {
            qustion: "你是1?".to_string(),
            anwser: "你猜1？".to_string(),
        };
        qa.qa.push(q1);
        qa.qa.push(q2);
        let json = serde_json::to_string(&qa).unwrap().to_string();
        println!("{}", json);
    }
}

pub mod request_json {
    use serde_yaml::Value;
    use serde::Deserialize;
    use serde::Serialize;

    // 构造Body
    pub mod request_body {
        use serde_yaml::Value;
        use serde::Deserialize;
        use serde::Serialize;

        #[derive(Serialize, Deserialize, Default, Debug)]
        pub struct ChatMessage {
            pub model: String,
            pub stream: bool,
            pub(crate) messages: Vec<Message>,
        }

        #[derive(Serialize, Deserialize, Default, Debug)]
        pub struct Message {
            pub(crate) role: String,
            pub(crate) content: String,
        }
    }

    // 收到的回复
    pub mod recv {
        use serde_yaml::Value;
        use serde::Deserialize;
        use serde::Serialize;


        //收到的请求
        #[derive(Serialize, Deserialize, Debug)]
        pub struct ChatCompletion {
            id: String,
            object: String,
            created: i64,
            model: String,
            usage: Usage,
            pub choices: Vec<Choice>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct Usage {
            prompt_tokens: i64,
            completion_tokens: i64,
            total_tokens: i64,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct Choice {
            pub message: Recv_Message,
            pub finish_reason: String,
            index: i64,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct Recv_Message {
            pub role: String,
            pub content: String,
        }
    }

    // 收到的stream回复
    pub mod recv_chunk {
        use serde_yaml::Value;
        use serde::Deserialize;
        use serde::Serialize;

        #[derive(Debug, Deserialize, Serialize)]
        pub struct ChatCompletionChunk {
            id: String,
            object: String,
            created: i64,
            model: String,
            pub choices: Vec<CompletionChoice>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        pub struct CompletionChoice {
            pub delta: Delta,
            index: usize,
            finish_reason: Option<String>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Delta {
            pub content: String,
        }
    }
}
