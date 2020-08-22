//#![windows_subsystem = "windows"]

extern crate web_view;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use web_view::*;

struct UserData {
	name: String,
}

use std::collections::HashMap;
use serde::{Serialize};

#[derive(Serialize)]
struct Message {
	headers: HashMap<String,String>,
	parts: Vec<Message>,
	ctype: String,
	body: String,
}

extern crate maildir;
use maildir::Maildir;
use mailparse::ParsedMail;

impl Message {
	fn from_parsed_mail(parsed: &ParsedMail<'_>) -> Self {
		let mut body: String = parsed.get_body().unwrap();
		if parsed.ctype.mimetype.starts_with("text/html") {
			body = sanitize(body);
		}
		Message {
			headers: parsed.headers.iter().map(|h| { (h.get_key(), h.get_value()) }).collect(),
			body: body,
			ctype: parsed.ctype.mimetype.clone(),
			parts: parsed.subparts.iter().map(|s| { Message::from_parsed_mail(s) }).collect(),
		}
	}
}

use html_sanitizer::TagParser;
fn sanitize(input: String) -> String {
	let mut tag_parser = TagParser::new(&mut input.as_bytes());
	tag_parser.walk(|tag| {
		if tag.name == "html" || tag.name == "body" {
			tag.ignore_self(); // ignore <html> and <body> tags, but still parse their children
		} else if tag.name == "head" || tag.name == "script" || tag.name == "style" {
			tag.ignore_self_and_contents(); // Ignore <head>, <script> and <style> tags, and all their children
		} else if tag.name == "a" {
			tag.allow_attribute(String::from("href")); // Allow specific attributes
		} else if tag.name == "img" {
			tag.rewrite_as(String::from("<b>Images not allowed</b>")); // Completely rewrite tags and their children
		} else {
			tag.allow_attribute(String::from("style")); // Allow specific attributes
		}
	})
}

use std::fs;
use std::io::prelude::*;
fn load_mail() -> String {
	let maildir = Maildir::from("E:/maildir/hotmail/INBOX");
	let mut entries = maildir.list_new();
	let entry = entries.last().unwrap().unwrap();
	//let parsed = entry.parsed().unwrap();

	let mut f = fs::File::open(entry.path()).unwrap();
	let mut d = Vec::<u8>::new();
	f.read_to_end(&mut d).unwrap();

	let parsed = mailparse::parse_mail(&d).unwrap();
	let msg = Message::from_parsed_mail(&parsed);
	let json = serde_json::to_string(&msg).unwrap();
	json
}

#[derive(Deserialize)]
#[serde(tag = "cmd")]
enum Cmd {
	LoadMail {},
	Browse { url: String },
	Exit {},
}

const BODY: &str = include_str!("../assets/body.html");
const MAIN_CSS: &str = include_str!("../assets/main.css");
const DARK_CSS: &str = include_str!("../assets/dark.css");
const JS: &str = include_str!("../assets/maildir.js");
fn main() {
	let webview = web_view::builder()
		.title("Mail time")
		.content(Content::Html(format!(r#"<!doctype html>
			<html>
				<head>
					<style>
						{styles}
					</style>
				</head>
				<body>
					<div id='app'>
						{body}
						<script type="text/javascript">
							{scripts}
						</script>
					</div>
				</body>
			</html>
		"#, body=BODY, styles=format!("{} {}", MAIN_CSS, DARK_CSS), scripts=JS)))
		.size(800, 600)
		.resizable(true)
		.debug(true)
		.user_data(UserData { name: "dctucker".to_string() })
		.invoke_handler(|webview, arg| {
            use Cmd::*;
            if let Ok(cmd) = serde_json::from_str(arg) {
				match cmd {
					LoadMail {} => {
						let headers = load_mail();
						let command = format!("setPreview({})", &headers);
						println!("{}", command);
						webview.eval(&command).unwrap();
					},
					Browse { url } => {
						println!("{}", url);
						webbrowser::open(&url).unwrap();
					},
					Exit {} => webview.exit(),
				};
			} else {
				eprintln!("Invalid command: {}", arg);
			}
			Ok(())
		})
	.build().unwrap();

	webview.run().unwrap();
}
