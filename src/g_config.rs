pub mod g_config{
    use serde::{Deserialize, Serialize};

    //一个带上下文的gpt机器人
    #[derive(Debug, Serialize, Deserialize)]
    pub struct bot_config {
        pub bot_config:config,
    }

    impl bot_config {
        //新建一个机器人
        pub fn new()->Self{
            //载入配置文件
            let config = config::load_config();
            return Self{
                bot_config: config,
            };
        }
    }


    #[derive(Debug, Serialize, Deserialize,Default)]
    pub struct config{
        pub endpoint:String,
        //token
        pub token:String,
        //检视编辑器
        pub view_editor:String,
        //对话记忆长度
        pub memory:u8,
        // 是否使用流式传递数据，如果启用可能会消耗更多的流量
        pub stream: bool,
        // prompt
        pub system_prompt:String,
    }

    impl config {
        pub fn load_config()->Self{
            //文件在$HOME/.config/gpt_bot/config.yaml
            let mut config_dir = dirs::config_dir().expect("Error: could not find user's config directory");
            config_dir.push("chat-gpt-line");
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir).expect("Error: could not create config directory");
            }
            config_dir.push("config.yaml");
            //如果文件不存在就创建一个
            if !std::path::Path::new(&config_dir).exists(){
                if let Ok(config)=std::fs::write(&config_dir,serde_yaml::to_string(&config{..Default::default()}).unwrap()){
                    println!("创建配置文件 {} 成功",config_dir.as_os_str().to_str().unwrap());
                    //退出程序
                    std::process::exit(0);
                }
            }
            //如果文件存在就读取
            let config_str = std::fs::read_to_string(&config_dir).unwrap();
            let yaml_config:config= serde_yaml::from_str(config_str.to_string().as_str()).unwrap();
            return yaml_config;
        }
    }
}
