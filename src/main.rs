//#![windows_subsystem = "windows"]

// extern crate tinyfiledialogs as tfd;
//use tfd::MessageBoxIcon;

extern crate web_view;
//extern crate serde_derive;
extern crate serde_json;

use web_view::*;

struct UserData {
	name: String,
}

extern crate maildir;
use maildir::Maildir;
fn load_mail() -> Vec<String> {
	let maildir = Maildir::from("E:/maildir/hotmail/INBOX");
	let mut entries = maildir.list_cur();
	let mut message = entries.next().unwrap().unwrap();
	let parsed = message.parsed().unwrap();
	let mut ret = vec![];
	for header in parsed.headers.iter() {
		ret.push(format!("{}: {}", header.get_key(), header.get_value()));
	}
	ret
}

fn main() {
	let webview = web_view::builder()
		.title("Mail time")
		.content(Content::Html(HTML))
		.size(800, 600)
		.resizable(true)
		.debug(true)
		.user_data(UserData { name: "dctucker".to_string() })
		.invoke_handler(|webview, arg| {
			match arg {
				"load_mail" => {
					let headers = load_mail().join("\n");
					webview.eval(&format!("setHeaders({})", serde_json::to_string(&headers).unwrap())).unwrap();
				},
				/*
				"open" => match tfd::open_file_dialog("Please choose a file...", "", None) {
					Some(path) => tfd::message_box_ok("File chosen", &path, MessageBoxIcon::Info),
					None => tfd::message_box_ok(
						"Warning",
						"You didn't choose a file.",
						MessageBoxIcon::Warning,
					),
				},
				"save" => match tfd::save_file_dialog("Save file...", "") {
					Some(path) => tfd::message_box_ok("File chosen", &path, MessageBoxIcon::Info),
					None => tfd::message_box_ok(
						"Warning",
						"You didn't choose a file.",
						MessageBoxIcon::Warning,
					),
				},
				"info" => {
					tfd::message_box_ok("Info", "This is a info dialog", MessageBoxIcon::Info)
				}
				"warning" => tfd::message_box_ok(
					"Warning",
					"This is a warning dialog",
					MessageBoxIcon::Warning,
				),
				"error" => {
					tfd::message_box_ok("Error", "This is a error dialog", MessageBoxIcon::Error)
				}
				*/
				"exit" => webview.exit(),
				_ => unimplemented!(),
			};
			Ok(())
		})
	.build().unwrap();

	webview.run().unwrap();
}

const HTML: &str = r#"
<!doctype html>
<html>
<head>
	<style>
		* {
			font-family: Arial;
			font-size: 12pt;
		}
		body {
			margin: 0px;
			padding: 0px;
		}
		#mail table {
			width: 100%;
			border-spacing: 0px;
			border-collapse: collapse;
		}
		#mail thead tr {
			background-color: #eee;
		}
		#mail thead tr th {
			text-align: left;
			color: #aaa;
		}
		#mail table tr * {
			border: 1px solid #eee;
		}
		#toolbar { display: none; }
		#headers {
		font-family: consolas;
		font-size: 10pt;
		white-space: pre;
		}
	</style>
</head>
<body>
	<div id="toolbar">
		<button onclick="external.invoke('load_mail')">Open</button>
		<button onclick="external.invoke('save')">Save</button>
		<button onclick="external.invoke('info')">Info</button>
		<button onclick="external.invoke('warning')">Warning</button>
		<button onclick="external.invoke('error')">Error</button>
		<button onclick="external.invoke('exit')">Exit</button>
	</div>
	<div id="mail">
		<table>
			<thead>
				<tr>
					<th>[]</th>
					<th>From</th>
					<th>Subject</th>
					<th>Received</th>
				</tr>
			</thead>
			<tbody>
				<tr>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
				</tr>
				<tr>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
				</tr>
				<tr>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
					<td>&nbsp;</td>
				</tr>
			</tbody>
		</table>
		<div id="preview">
			<div id="headers">
			</div>
		&nbsp;
		</div>
	</div>
	<script type="text/javascript">
	function setHeaders(data) {
		console.log("Hello");
		document.getElementById("headers").innerHTML = data.replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/&/g,'&amp;');
	}
	var callback = function(){
	  // Handler when the DOM is fully loaded
		external.invoke('load_mail');
	};

	if (
		document.readyState === "complete" ||
		(document.readyState !== "loading" && !document.documentElement.doScroll)
	) {
	  callback();
	} else {
	  document.addEventListener("DOMContentLoaded", callback);
	}

	</script>
</body>
</html>
"#;
