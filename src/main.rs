//#![windows_subsystem = "windows"]

const MAILDIR_PATH: &str = "E:/Maildir";

extern crate actix_rt;
extern crate actix_web;
extern crate futures;
extern crate mime_guess;
extern crate rust_embed;

extern crate chrono;
use chrono::prelude::*;

#[macro_use]
extern crate cached;
use cached::SizedCache;

extern crate web_view;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use web_view::*;

use std::collections::HashMap;
use serde::{Serialize};

type MessageHeaders = HashMap<String,String>;

#[derive(Serialize, Clone)]
pub struct Message {
	headers: MessageHeaders,
	parts: Vec<Message>,
	ctype: String,
	body: Vec<u8>,
}

extern crate maildir;
use mailparse::ParsedMail;

impl Message {
	fn from_parsed_mail(parsed: &ParsedMail<'_>) -> Self {
		Message {
			headers: parsed.headers.iter().map(|h| { (h.get_key(), h.get_value()) }).collect(),
			body: if parsed.ctype.mimetype.starts_with("text/html") {
					sanitize(parsed.get_body().unwrap()).into_bytes()
				} else {
					parsed.get_body_raw().unwrap()
				},
			ctype: parsed.ctype.mimetype.clone(),
			parts: parsed.subparts.iter().map(|s| { Message::from_parsed_mail(s) }).collect(),
		}
	}
	fn skeleton(&self) -> Message {
		Message {
			headers: self.headers.clone(),
			ctype: self.ctype.clone(),
			parts: self.parts.iter().map(|s| s.skeleton() ).collect(),
			body: vec![],
		}
	}
}

fn format_filename(s: String, full_path: &str) -> String {
	s.replace("\\","/")
		.replace(full_path,"")
		.replace("\u{f022}",":")
}
fn format_date(s: String) -> String {
	let date = mailparse::dateparse(&s).unwrap();
	let date: DateTime<Local> = Utc.timestamp(date, 0).into();
	date.format("%Y-%m-%d %H:%M:%S").to_string()
}
fn format_headers(parsed: Vec<mailparse::MailHeader>, new: usize) -> HashMap<String,String> {
	let mut headers = parsed.iter().filter(|h| match h.get_key().as_str() {
			"From" | "Date" | "Subject" => true,
			_ => false,
		})
		.map(|h| { (h.get_key(), h.get_value()) })
		.collect::<HashMap<String,String>>();
	*(headers.get_mut("Date").unwrap()) = format_date(headers["Date"].clone());
	headers.entry("new".to_string()).or_insert(format!("{}", new));
	headers
}
fn map_messages(list: maildir::MailEntries, full_path: String, new: usize) -> HashMap<String,HashMap<String,String>> {
	list.map(|e| {
		let mut e = e.unwrap();
		let real_path = e.path();
		let path = format_filename(real_path.display().to_string(), &full_path);
		let parsed = e.headers().unwrap();
		let headers = format_headers(parsed, new);
		(path, headers)
	}).collect::<HashMap<_,_>>()
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
			.map(|e| format_filename(e.unwrap().into_path().display().to_string(), full_path))
			.filter(|s| ! (s.ends_with("/new") || s.ends_with("/cur") || s.ends_with("/tmp") || s.len() == 0) )
			.collect::<Vec<String>>();
		self
	}
	fn set_current_mailbox(&mut self, path: String) -> &Self {
		let full_path = format!("{}/{}", MAILDIR_PATH, path);
		let dir = maildir::Maildir::from(full_path.clone());
		self.messages = map_messages( dir.list_new(), full_path.clone(), 1 );
		self.messages.extend( map_messages( dir.list_cur(), full_path.clone(), 0 ) );
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

cached_result!{
	MESSAGES: SizedCache<String, Message> = SizedCache::with_size(50);
	fn load_message(path: String) -> Result<Message, ()> = {
		let path = path.replace(":", "\u{f022}");
		let path = format!("{}/{}", MAILDIR_PATH, path);
		if let Ok(mut f) = fs::File::open(path.clone()) {
			let mut d = Vec::<u8>::new();
			f.read_to_end(&mut d).unwrap();
			let parsed = mailparse::parse_mail(&d).unwrap();
			let msg = Message::from_parsed_mail(&parsed);
			Ok(msg)
		} else {
			eprintln!("Unable to open {}", path.clone());
			Err(())
		}
	}
}

fn traverse_message<'a>(msg: &'a Message, loc: &[usize]) -> Result<&'a Message, ()> {
	if loc.len() == 1 {
		return Ok(&msg.parts[loc[0]]);
	}
	traverse_message(&msg.parts[loc[0]], &loc[1..])
}


fn get_mail(req: HttpRequest) -> HttpResponse {
	let path = req.match_info().get("path").unwrap();
	if let Ok(msg) = load_message(path.to_string()) {
		let query = req.query_string();
		if query.len() == 0 {
			println!("{}", serde_json::to_string_pretty(&msg.skeleton().parts).unwrap());
			return HttpResponse::Ok().json(msg.skeleton());
		} else if query == "," {
			return HttpResponse::Ok()
				.content_type(msg.ctype.clone())
				.body(msg.body.clone())
		}
		println!("query = {}", query.clone());
		let loc = query.split(",").map(|e| usize::from_str_radix(e, 10).unwrap()).collect::<Vec<usize>>();
		if let Ok(m) = traverse_message(&msg, &loc) {
			HttpResponse::Ok()
				.content_type(m.ctype.clone())
				.body(m.body.clone())
		} else {
			eprintln!("404 traversal {}", path);
			HttpResponse::NotFound().body("Not Found")
		}
	} else {
		eprintln!("404 Unable to load message {}", path.to_string());
		HttpResponse::NotFound().body("Not Found")
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
	let user_data = UserData::new();

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
		.user_data(user_data)
		.invoke_handler(|webview, arg| {
			use Cmd::*;
			if let Ok(cmd) = serde_json::from_str(arg) {
				let data = webview.user_data_mut();
				match cmd {
					Init {} => {
						render(webview).unwrap();
					},
					LoadMail {} => {},
					SetMailbox { path } => {
						data.set_current_mailbox(path);
						webview.eval(&format!("rpc.render({})", serde_json::json!({
							"current_mailbox": webview.user_data().current_mailbox,
							"messages": webview.user_data().messages,
						}))).unwrap();
					},
					Browse { url } => {
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

	let _ = server.stop(true).wait();
}
