#![allow(warnings)]
mod g_bot;
mod g_config;

use std::any::Any;
use std::fmt::Debug;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use tokio::{spawn, task};
use tokio::runtime::Runtime;
use serde_json::{json, Value};
use clap::{Arg};
use std::path::PathBuf;
use std::fs;
use std::future::IntoFuture;
use std::hash::Hash;
use std::io::{BufRead, Read, Write};
use std::sync::mpsc;
use std::sync::mpsc::{channel, Receiver};
use tokio::sync::mpsc::Sender;
use bytes::{Buf, Bytes};
use serde::de::Unexpected::Option;
use serde_yaml::Index;
use tokio::io::AsyncSeek;
use crate::g_bot::history::QA;
use crate::g_config::g_config::bot_config;
//use nix::unistd::{getppid};


#[tokio::main]
async fn main() {
    let mut content_string=Vec::new();
    for x in std::env::args() {
        content_string.push(x);
    }

    let mut question=String::new();
    for index in 1..content_string.len(){
        question.push_str(content_string[index].as_str());
    }

    let mut bot= bot_config::new();

    let mut qa:QA=QA::default();


    if bot.bot_config.stream {
        let view_editor=bot.bot_config.view_editor.clone();
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<String>(32);
        qa.qustion=question.clone().to_string();

        tokio::spawn(async move {
            //bot.chat_completion_stream(question.as_str(),sender).await;
            let mut b=crate::g_bot::g_bot::g_bot::new();
            b.send_qustion(question,sender).await;

        });

        if !bot.bot_config.view_editor.is_empty() {
            let mut cmd=std::process::Command::new(view_editor)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::inherit())
                .spawn()
                .unwrap();
            let mut stdin=cmd.stdin.take().unwrap();

            while let Some(recv) = receiver.recv().await{

                qa.anwser.push_str(recv.clone().as_str());
                stdin.write_all(recv.to_string().as_bytes()).unwrap();

                stdin.flush().unwrap();
            }
        }else {
            while let Some(recv) = receiver.recv().await{
                qa.anwser.push_str(recv.clone().as_str());
                print!("{}",recv);
                std::io::stdout().flush().unwrap();
            }
        }



    }else {
        let mut b=crate::g_bot::g_bot::g_bot::new();
        let (tx,mut rx)=tokio::sync::mpsc::channel(10);

        b.send_qustion(question,tx);
        if let Some(q) = rx.recv().await{
            qa.anwser=q.clone();
            if !bot.bot_config.view_editor.is_empty() {
                show_with_editor(bot.bot_config.view_editor,q);
            }else {
                println!("{}",q);
            }
        }


    }
    //保存上下文
    g_bot::history::History::new()
        .write_to_history_content(qa);

}
fn show_with_editor(editor_name:String,answer:String){

    if !editor_name.is_empty() {
        let pipe=std::process::Stdio::piped();

        let mut cmd=std::process::Command::new(editor_name)
            .stdin(pipe)
            .spawn()
            .unwrap();
        let mut stdin=cmd.stdin.take().unwrap();
        stdin.write_all(answer.as_str().as_bytes()).unwrap();

    }
}

