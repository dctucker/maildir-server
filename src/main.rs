//#![windows_subsystem = "windows"]

extern crate actix_rt;
extern crate actix_web;
extern crate futures;
extern crate mime_guess;
extern crate rust_embed;

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
fn load_mail() -> Message {
	let maildir = Maildir::from("E:/maildir/hotmail/INBOX");
	let entries = maildir.list_new();
	let entry = entries.last().unwrap().unwrap();
	//let parsed = entry.parsed().unwrap();

	let mut f = fs::File::open(entry.path()).unwrap();
	let mut d = Vec::<u8>::new();
	f.read_to_end(&mut d).unwrap();

	let parsed = mailparse::parse_mail(&d).unwrap();
	let msg = Message::from_parsed_mail(&parsed);
	msg
}

#[derive(Deserialize)]
#[serde(tag = "cmd")]
enum Cmd {
	LoadMail {},
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

fn get_mail(req: HttpRequest) -> HttpResponse {
	let _path = if req.path() == "/" {
		"index.html"
	} else {
		&req.path()[1..] // trim leading '/'
	};

	HttpResponse::Ok()
		//.content_type("application/json; charset=utf8")
		.json(load_mail())


	/*
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
	*/
}

fn list_boxes() -> HttpResponse {
	let full_path = "E:/Maildir";
	let ret: Vec<String> = walkdir::WalkDir::new(full_path.clone())
		.into_iter()
		.filter_entry(|e| e.file_type().is_dir())
		.map(|e| e.unwrap().into_path().display().to_string().replace("\\","/").replace(full_path,""))
		.collect();
	HttpResponse::Ok()
		.json(ret)
}

fn mailbox(path: web::Path<String>) -> HttpResponse {
	let full_path = format!("E:/Maildir/{}", path);
	let ret: Vec<String> = walkdir::WalkDir::new(full_path.clone())
		.into_iter()
		.map(|e| e.unwrap().into_path().display().to_string().replace("\\","/").replace(&full_path, ""))
		.collect();
	HttpResponse::Ok()
		.json(ret)
}

fn main() {
	let (server_tx, server_rx) = mpsc::channel();
	let (port_tx, port_rx) = mpsc::channel();

	// start actix web server in separate thread
	thread::spawn(move || {
		let sys = actix_rt::System::new("actix-example");

		let server = HttpServer::new(|| {
				App::new()
					.route("/mail/message.json", web::get().to(get_mail))
					//.route("/mail/message/{msg_id}/headers.json", web::get().to(mail_headers))
					.route("/mail/boxes", web::get().to(list_boxes))
					.route("/mail/box/{dir:.*}", web::get().to(mailbox))
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
		.user_data(UserData { name: "dctucker".to_string() })
		.invoke_handler(|webview, arg| {
            use Cmd::*;
            if let Ok(cmd) = serde_json::from_str(arg) {
				match cmd {
					LoadMail {} => {
						/*
						let headers = load_mail();
						let command = format!("setPreview({})", &headers);
						//println!("{}", command);
						webview.eval(&command).unwrap();
						*/
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
