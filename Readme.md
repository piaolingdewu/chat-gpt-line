# 简述

## 使用

1. 克隆本项目到本地
    ```bash
   git clone https://github.com/piaolingdewu/chat-gpt-line.git
    ```

2. 进入文件夹
    ```bash
    cd chat-gpt-line/
    ```

3. 使用cargo进行编译
    ```bash
    cargo build --release
    ```
4. 将编译好的二进制文件移动到`/usr/bin`或者`/usr/local/bin`中
    ```bash
    sudo mv target/release/chat-gpt-line /usr/bin
    ```
5. 按照配置文件的要求配置好文件

 
6. 运行
    ```bash
    chat-gpt-line '这是一个测试，请回答测试成功!'
    ```



## 配置文件

配置文件在 `~/.config/chat-gpt-line/config.yaml`，默认配置如下：
```yaml
# 配置代理
http_proxy: ''
https_proxy: ''
# openai的api key
token: ''
# 用于查看编辑器 (默认为cat) 可以使用glow或bat来高亮代码(会有一些bug)
view_editor: cat
# 启用上下文长度
memory: 1
# 使用的模型名称
module: gpt-3.5-turbo
# 是否使用stream推送
stream: true
```

# 其他
## 上下文
上下文的记录在 `~/.config/chat-gpt-line/History/`，文件夹下。

记录的方式是获取你当前使用的命令行的进程id，使其作为上下文的文件名，然后将上下文写入文件中。

## 