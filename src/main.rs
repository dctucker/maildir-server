//#![windows_subsystem = "windows"]

const MAILDIR_PATH: &str = "E:/Maildir";

extern crate actix_rt;
extern crate actix_web;
extern crate futures;
extern crate mime_guess;
extern crate rust_embed;

extern crate chrono;
use chrono::prelude::*;

extern crate web_view;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use web_view::*;

use std::collections::HashMap;
use serde::{Serialize};

type MessageHeaders = HashMap<String,String>;

#[derive(Serialize)]
struct Message {
	headers: MessageHeaders,
	parts: Vec<Message>,
	ctype: String,
	body: String,
}

extern crate maildir;
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

#[derive(Debug,Serialize)]
struct UserData {
	mailboxes: Vec<String>,
	current_mailbox: String,
	messages: HashMap<String,MessageHeaders>,
	current_message: String,
}
impl UserData {
	fn new() -> UserData {
		UserData {
			current_mailbox: "".to_string(),
			current_message: "".to_string(),
			mailboxes: vec![],
			messages: HashMap::new(),
		}.load_mailboxes()
	}
	fn load_mailboxes(mut self) -> Self {
		let full_path = MAILDIR_PATH.clone();
		self.mailboxes = walkdir::WalkDir::new(full_path.clone())
			.into_iter()
			.filter_entry(|e| e.file_type().is_dir())
			.map(|e| e.unwrap().into_path().display().to_string()
				.replace("\\","/")
				.replace(full_path,"")
				.replace("\u{f022}",":"))
			.collect::<Vec<String>>();
		self
	}
	fn set_current_mailbox(&mut self, path: String) -> &Self {
		let full_path = format!("{}/{}", MAILDIR_PATH, path);
		self.messages = walkdir::WalkDir::new(full_path.clone())
			.min_depth(1)
			.max_depth(2)
			.into_iter()
			.filter_entry(|e| e.file_type().is_file())
			.map(|e| {
				let e = e.unwrap();
				let real_path = e.into_path();
				let path = real_path.display().to_string()
					.replace("\\","/")
					.replace(&full_path, "")
					.replace("\u{f022}",":");

				let mut headers = HashMap::new();
				match fs::File::open(real_path.clone()) {
					Ok(mut f) => {
						let mut d = Vec::<u8>::new();
						f.read_to_end(&mut d).unwrap();
						let (parsed, _) = mailparse::parse_headers(&d).unwrap();
						headers = parsed.into_iter()
							.filter(|h| match h.get_key().as_str() {
								"From" | "Date" | "Subject" => true,
								_ => false,
							})
							.map(|h| { (h.get_key(), h.get_value()) })
							.collect();
						let date = mailparse::dateparse(&headers["Date"]).unwrap();
						let date: DateTime<Local> = Utc.timestamp(date, 0).into();
						let date = date.format("%Y-%m-%d %H:%M:%S").to_string();
						*(headers.get_mut("Date").unwrap()) = date;
					},
					_ => {
						eprintln!("Couldn't open file {}", real_path.clone().display());
					}
				};

				(path, headers)
			})
			.collect::<HashMap<_,_>>();
		self.current_mailbox = path;
		self
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
			tag.allow_attribute(String::from("src"));
			tag.allow_attribute(String::from("width"));
			tag.allow_attribute(String::from("height"));
			//tag.rewrite_as(String::from("<b>Images not allowed</b>")); // Completely rewrite tags and their children
		} else {
			tag.allow_attribute(String::from("style")); // Allow specific attributes
		}
	})
}

use std::fs;
use std::io::prelude::*;

#[derive(Deserialize)]
#[serde(tag = "cmd")]
enum Cmd {
	Init {},
	LoadMail {},
	SetMailbox { path: String },
	Browse { url: String },
	Exit {},
}


use std::{borrow::Cow, sync::mpsc, thread};
use rust_embed::RustEmbed;
use actix_web::{body::Body, web, App, HttpRequest, HttpResponse, HttpServer};
use futures::future::Future;

#[derive(RustEmbed)]
#[folder = "assets"]
struct Asset;

fn assets(req: HttpRequest) -> HttpResponse {
	let path = if req.path() == "/" {
		"index.html"
	} else {
		&req.path()[1..] // trim leading '/'
	};

	// query the file from embedded asset with specified path
	match Asset::get(path) {
		Some(content) => {
			let body: Body = match content {
				Cow::Borrowed(bytes) => bytes.into(),
				Cow::Owned(bytes) => bytes.into(),
			};
			HttpResponse::Ok()
				.content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
				.body(body)
		}
		None => HttpResponse::NotFound().body("404 Not Found"),
	}
}

fn get_mail(path: web::Path<String>) -> HttpResponse {
	let path = path.to_string().replace(":", "\u{f022}");
	let path = format!("{}/{}", MAILDIR_PATH, path);
	match fs::File::open(path.clone()) {
		Ok(mut f) => {
			let mut d = Vec::<u8>::new();
			f.read_to_end(&mut d).unwrap();
			let parsed = mailparse::parse_mail(&d).unwrap();
			let msg = Message::from_parsed_mail(&parsed);
			HttpResponse::Ok()
				//.content_type("application/json; charset=utf8")
				.json(msg)
		},
		_ => {
			eprintln!("404 {}", path);
			HttpResponse::NotFound().body("Not Found")
		},
	}
}

fn render(webview: &mut WebView<UserData>) -> WVResult {
    let call = {
        let data = webview.user_data();
        println!("{:#?}", data);
        format!("rpc.render({})", serde_json::to_string(data).unwrap())
    };
    webview.eval(&call)
}

fn main() {
	let (server_tx, server_rx) = mpsc::channel();
	let (port_tx, port_rx) = mpsc::channel();

	// start actix web server in separate thread
	thread::spawn(move || {
		let sys = actix_rt::System::new("actix-example");

		let server = HttpServer::new(|| {
				App::new()
					//.route("/mail/message.json", web::get().to(get_mail))
					//.route("/mail/box/{dir:.*}/{msg_id}/headers.json", web::get().to(mail_headers))
					.route("/mail/messages/{path:.*}", web::get().to(get_mail))
					/*.route("/mail/boxes", web::get().to(|| {
						HttpResponse::Ok()
							.json()
					}))*/
					//.route("/mail/box/{dir:.*}", web::get().to(mailbox))
					.route("*", web::get().to(assets))
			})
			.bind("127.0.0.1:0")
			.unwrap();

		let port = server.addrs().first().unwrap().port();
		let server = server.start();

		let _ = port_tx.send(port);
		let _ = server_tx.send(server);
		let _ = sys.run();
	});

	let port = port_rx.recv().unwrap();
	let server = server_rx.recv().unwrap();
	let webview = web_view::builder()
		.title("Mail time")
		.content(Content::Url(format!("http://127.0.0.1:{}", port)))
		.size(1024, 768)
		.resizable(true)
		.debug(true)
		.user_data(UserData::new())
		.invoke_handler(|webview, arg| {
			use Cmd::*;
			if let Ok(cmd) = serde_json::from_str(arg) {
                let data = webview.user_data_mut();
				match cmd {
					Init {} => {
					},
					LoadMail {} => {
						/*
						   let headers = load_mail();
						   let command = format!("setPreview({})", &headers);
						//println!("{}", command);
						webview.eval(&command).unwrap();
						*/
					},
					SetMailbox { path } => {
						data.set_current_mailbox(path);
					},
					Browse { url } => {
						webbrowser::open(&url).unwrap();
					},
					Exit {} => webview.exit(),
				};
			} else {
				eprintln!("Invalid command: {}", arg);
			}
			render(webview)
		})
	.build().unwrap();

	webview.run().unwrap();

    let _ = server.stop(true).wait();
}
