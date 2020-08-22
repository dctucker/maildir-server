var callback = function(){
	external.invoke('load_mail');

	document.addEventListener('mouseover', function(event) {
		var hoveredEl = event.target; // The actual element which was hovered.
		if (hoveredEl.tagName !== 'A') { return; } // Ignore non links
		console.log(hoveredEl.href); // Do what we want here!
	});
};

if (
	document.readyState === "complete" ||
	(document.readyState !== "loading" && !document.documentElement.doScroll)
) {
  callback();
} else {
  document.addEventListener("DOMContentLoaded", callback);
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
	document.getElementById("headers").innerHTML = formatHeaders(data.headers);
	document.getElementById("body").innerHTML = formatBody(data);
}
