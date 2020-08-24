//'use strict';
rpc = {
	invoke: function(cmd, args) {
		if( args === undefined ){
			args = {};
		}
		args.cmd = cmd;
		external.invoke(JSON.stringify(args));
	},
	init : function() { rpc.invoke('Init'); },
	render: function(data) {
		document.getElementById("mailbox_list").innerHTML = formatMailboxList(data.mailboxes);
		document.getElementById("mail").getElementsByTagName("tbody")[0].innerHTML = formatMessages(data.messages);
		rpc.user_data = data;
	},
	setMailbox: (i) => {
		box = rpc.user_data.mailboxes[i];
		rpc.invoke("SetMailbox", {path: box});
		rpc.user_data.current_mailbox = box;
	},
};

window.onload = () => {
	rpc.init();
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
	fetch('/mail/messages/hotmail/Inbox/cur/1597719151_1.1.99720b41fab8,U=176,FMD5=3882d32c66e7e768145ecd8f104b0c08:2,S')
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
	if( s === undefined ){
		return "N/A";
	}
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

function formatMailboxList(list) {
	html = "";
	for( i in list ){
		box = list[i];
		html += "<li onclick='rpc.setMailbox("+i+")'>";
		html += box;
		html += "</li>";
	}
	return html;
}

function formatMessages(messages) {
	html = "";
	for( i in messages ){
		m = messages[i];
		html += "<tr>";
		html += "<td><input type='checkbox' value='"+i+"' /></td>";
		html += "<td>" + escapeHtml(m.From   ) + "</td>";
		html += "<td>" + escapeHtml(m.Subject) + "</td>";
		html += "<td>" + escapeHtml(m.Date   ) + "</td>";
		html += "</tr>";
	}
	return html;
}

function setPreview(data) {
	document.getElementById("headers_label").innerHTML = data.headers["From"] + " &mdash; " + data.headers["Subject"];
	document.getElementById("headers").innerHTML = formatHeaders(data.headers);
	document.getElementById("body").innerHTML = formatBody(data);
}
