//'use strict';
rpc = {
	invoke: function(cmd, args) {
		if( args === undefined ){
			args = {};
		}
		args.cmd = cmd;
		external.invoke(JSON.stringify(args));
	},
};

var content_loaded = function(){
	document.addEventListener('mouseover', function(event) {
		var hovered = event.target; // The actual element which was hovered.
		if (hovered.tagName !== 'A') {
			document.getElementById("href").innerHTML = "";
			return;
		}
		//console.log(hovered.href); // Do what we want here!
		document.getElementById("href").innerHTML = escapeHtml(hovered.href);
	});

	/*
	document.addEventListener('click', function(event) {
		if (event.target.tagName !== 'A') {
			return;
		}
		rpc.invoke('Browse', { url: event.target.href });
		return false;
	});
	*/

	document.onclick = function(event) {
		event = event || window.event;
		var element = event.target || event.srcElement;
		if( element.tagName == "A" ){
			rpc.invoke('Browse', { url: element.href });
			return false;
		}
	}

	//rpc.invoke('LoadMail');
	fetch('/mail/message.json')
		.then(resp => resp.json())
		.then(data => setPreview(data));

};


if (
	document.readyState === "complete" ||
	(document.readyState !== "loading" && !document.documentElement.doScroll)
) {
  content_loaded();
} else {
  document.addEventListener("DOMContentLoaded", content_loaded);
}

function escapeHtml(s) {
	return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

function formatBody(part) {
	html = "<hr />";
	if( part.headers["Content-Type"].startsWith("text/html") ) {
		html += "<div class='html'>"+part.body+"</div>"; // risky
	} else if( part.headers["Content-Type"].startsWith("text/plain") ) {
		html += "<section class='accordion'>";
		html += "<input type='checkbox' name='collapse' id='handle1'>";
		html += "<label for='handle1'>Content-Type: text/plain</label>";
		html += "<div class='content plain'>" + part.body + "</div>";
		html += "</section>";
	} else {
		html += escapeHtml(part.body);
	}
	for( p in part.parts ){
		html += "<div class='part'>" + formatBody(part.parts[p]) + "</div>";
	}
	return html;
}

function formatHeaders(headers) {
	html = "";
	keys = Object.keys(headers).sort();
	for( h in keys ){
		val = escapeHtml(headers[keys[h]]);
		if( keys[h][0] == "x" || keys[h][0] == "X" ){
			html += "<tr><th>"+keys[h]+": </th><td>"+val+"</td></tr>";
		} else {
			html += "<tr><th style='text-align:right'>"+keys[h]+": </th><td>"+val+"</td></tr>";
		}
	}
	return html;
}

function setPreview(data) {
	document.getElementById("headers_label").innerHTML = data.headers["From"] + " &mdash; " + data.headers["Subject"];
	document.getElementById("headers").innerHTML = formatHeaders(data.headers);
	document.getElementById("body").innerHTML = formatBody(data);
}
