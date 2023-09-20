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

        pub async fn send_qustion(&mut self, quest: String, tx: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Sender<String>>>) {
            //载入历史记录
            let mut history = vec![];
            if let Some(load) = History::new().read_history_content(self.config.memory) {
                history = load;
            }

            // 这里临时修改一下参数
            super::gpt_request::gpt_request::new()
                .add_token(self.config.token.clone())
                .add_QA_histore(history)
                .enable_stream(self.config.stream.clone())
                .set_qustion(quest)
                .set_endpoint(self.config.endpoint.clone())
                .set_system_prompt(self.config.system_prompt.clone()) //设置prompt
                .send(tx).await;
        }
    }

    #[tokio::test]
    async fn bot_test() {
        //发送简单的请求
        let mut bot = g_bot::new();

        println!("{:?}", bot.config);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
        let g_tx=std::sync::Arc::new(tokio::sync::Mutex::new(tx));

        tokio::spawn(async move {
            bot.send_qustion("帮我使用rust编写一个hello world程序".to_string(), g_tx).await;
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
    use std::borrow::BorrowMut;
    use std::cell::RefCell;
    use std::default;
    use std::fmt::Debug;
    use std::io::BufRead;
    use reqwest::{Client, RequestBuilder};
    use serde::Serialize;
    use std::sync::Arc;
    use std::sync::Mutex;
    use telegram_bot::Message;
    use crate::g_bot::history::QA;

    use crate::g_bot::request_json::*;
    use crate::g_bot::request_json::recv_chunk::ChatCompletionChunk;
    use crate::g_config;

    #[derive(Default)]
    pub struct gpt_request {
        // endpoint
        endpoint: String,
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
        system_prompt: String,
    }

    // 流式分析
    pub struct stream_data_slover{
        in_buf:Vec<u8>,
        out_buf:Vec<String>
    }
    impl stream_data_slover {
        
        pub fn new()->Self{
            Self{
                in_buf:[].to_vec(),
                out_buf:[].to_vec()
            }
        }
        pub fn try_conver_json(&self,json_string:String)->Option<String>{
            let rp_string=json_string.replace("data:", "");

            if let Ok(json_data) = serde_json::from_str::<super::request_json::recv_chunk::ChatCompletionChunk>(&rp_string) {
                // 解析成功
                if json_data.choices.len()>0{
                    let out_string = json_data.choices[0].delta.content.clone();
                    return Some(out_string)
                }
            }
            None
        }
        // 添加byte数据
        pub fn add_byte_line(&mut self,bytes_line:&Vec<u8>){
            
            // 对传入的内容进行解析
            // 尝试解析为json
            match String::from_utf8(bytes_line.clone()) {
                Ok(line_string)=>{
                    // 尝试解析成json
                    match self.try_conver_json(line_string) {
                        Some(out_string) => {
                            // 解析成功
                            self.out_buf.insert(0, out_string);
                            
                            if self.in_buf.len()>0{
                                self.in_buf.clear();
                            }
                        },
                        None => {
                            // 解析失败
                            self.in_buf.append(&mut bytes_line.clone());
                        },
                    }

                    
                }
                Err(_)=>{
                    // 解析为string失败，这个时候大概率是一个中文两个字节，正好解析到中间了 放入缓冲区中
                    self.in_buf.append(&mut bytes_line.clone());
                }
            }

            //尝试解析in_buf的内容
            if self.in_buf.len()>0{
                let json_string=String::from_utf8(self.in_buf.clone());
                
                if let Ok(json_string) =json_string{
                    let l=json_string.clone();
                    let c_json=json_string.split_once("data:").unwrap_or(("",l.as_str()));

                    //println!("buf:{}",c_json.1);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&c_json.1){

                        if let Some(out_string) = self.try_conver_json(json_string) {
                            // 解析成功
                            self.out_buf.insert(0, out_string);
                        }
                        self.in_buf.clear();
                    }
                }
            }

        }

        pub fn pop_slover_line(&mut self)->Option<String>{
            //
            self.out_buf.pop()
        }
        
    }

    #[test]
    fn stream_data_test(){
        match std::fs::read("t1.txt") {
            Ok(content) => {
                let mut s_slo=stream_data_slover::new();

                for i in content.lines(){
                    let mut line=i.unwrap().as_bytes().to_vec();
                    s_slo.add_byte_line(&line);
                    if let Some(out_string) = s_slo.pop_slover_line() {
                        print!("{}",out_string);
                    }
                }
            },
            Err(err) => {
                println!("未读取到文件内容")
            },
        }    


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
        pub fn set_system_prompt(&mut self, system_prompt: String) -> &mut gpt_request {
            self.system_prompt=system_prompt;
            return self
        }
        pub fn set_endpoint(&mut self,endpoint:String)-> &mut gpt_request{
            self.endpoint=endpoint;
            return self;
        }

        pub fn get_stream_data(&self,stream_data:String)->Option<String>{
            todo!()
        }


        


        //发送请求
        pub async fn send(&mut self, mut sender: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Sender<String>>>) {
            let mut body = super::request_json::request_body::ChatMessage { ..Default::default() };
            let mut sender_arc=std::sync::Arc::new(std::sync::Mutex::new(&sender));
            body.model = self.model.clone();
            body.stream = self.stream.clone();
            
            // 读取prompt
            let system_prompt=if self.system_prompt.is_empty(){
                "".to_string()
            }else{self.system_prompt.clone()};
            
            // 设置prompt
            body.messages.push(super::request_json::request_body::Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            });

            // 设置基础内容
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
            //println!("prompt:{}",serde_json::to_string(&body.messages).unwrap());



            let http_proxy = self.http_proxy.clone();
            let token = self.token.clone();
            let is_stream = self.stream.clone();

            let mut request=if http_proxy.is_empty() { Client::new() }else {
                Client::builder().proxy(reqwest::Proxy::http(&http_proxy).unwrap()).build().unwrap()
            };
            match request.post(self.endpoint.clone())
                         .header("Content-Type", "application/json")
                         .header("api-key", format!("{}", token.clone()))
                         .body(serde_json::to_string(&body).unwrap())
                         .send().await {
                Ok(mut client) => {

                    // 请求成
                    //println!("{}",serde_json::to_string(&body).unwrap());

                    if is_stream {
                        
                        let mut fail_buf=Arc::new(tokio::sync::Mutex::new(Vec::<u8>::new()));
                        //println!("在发送阶段");

                        while let Some(Recv) = client.chunk().await.unwrap() {
                            //解析chunk并发送
                            let mut stream=std::io::BufReader::new(Recv.as_ref());
                            for line_byte in stream.split(b'\n') {
                                
                                let exp_string_data=|json_string:String|->Option<String>{
                                    if let Ok(json_data) = serde_json::from_str::<super::request_json::recv_chunk::ChatCompletionChunk>(&json_string) {
                                        // 解析成功
                                        if json_data.choices.len()>0{
                                            let out_string = json_data.choices[0].delta.content.clone();
                                            return Some(out_string)
                                        }
                                    }
                                    None
                                };

                                let try_exp_fail_buf=||->Option<String>{
                                    let mut  guard=fail_buf.try_lock().unwrap();
                                    
                                    let data_bytes=guard.clone();

                                    let mut json_string=String::from_utf8( data_bytes).unwrap_or_default();
                                    json_string=json_string.replace("\n", "");
                                    match serde_json::from_str::<super::request_json::recv_chunk::ChatCompletionChunk>(&json_string) {
                                        Ok(json_data) => {
                                            // 解析成功
                                            guard.clear();
                                            if json_data.choices.len()>0{
                                                let out_string = json_data.choices[0].delta.content.clone();
                                                return Some(out_string)
                                            }

                                        },
                                        Err(_) => {
                                            // 解析失败
                                        },
                                    }
                                    
                                    None
                                };


                                if let Ok(line_byte) = line_byte {

                                    // @TODO:解析出来的内容发送给sender
                                    // sender.send(out_string).await.unwrap();
                                    let mut fail_buff_guard=fail_buf.try_lock().unwrap();
                                    match String::from_utf8(line_byte.clone()) {
                                        Ok(line_string)=>{
                                            // 尝试解析成json
                                            let rp_string=line_string.replace("data:", "");

                                            if let Some(revice_content) =  exp_string_data(rp_string){

                                                // 解析成功，发送数据


                                                if fail_buff_guard.len()>0{
                                                    fail_buff_guard.clear()
                                                }
                                                let mut guard=sender.borrow_mut().try_lock().unwrap();
                                                guard.send(revice_content).await.unwrap();
                                                //sender.send(revice_content).await.unwrap();

                                            }else{
                                                let mut li_co=line_byte.clone();


                                                fail_buff_guard.append(&mut li_co);

                                                if let Some(recv_content) = try_exp_fail_buf() {
                                                    let mut guard=sender.borrow_mut().try_lock().unwrap();
                                                    guard.send(recv_content).await.unwrap();
                                                }
                                            }
                                            //let json_data=serde_json::from_str(&line_string);
                                        }
                                        Err(_)=>{
                                            // 解析为string失败，这个时候大概率是一个中文两个字节，正好解析到中间了
                                            let mut li_co=line_byte.clone();
                                            
                                            fail_buff_guard.append(&mut li_co);
                                            if let Some(recv_content) = try_exp_fail_buf() {
                                                let mut guard=sender.borrow_mut().try_lock().unwrap();
                                                guard.send(recv_content).await.unwrap();
                                            }
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

                            let mut guard=sender.borrow_mut().try_lock().unwrap();
                            guard.send(rec.choices[0].message.content.clone()).await.unwrap();

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

