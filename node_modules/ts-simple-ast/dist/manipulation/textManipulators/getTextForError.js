"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getTextForError(newText, pos, length) {
    if (length === void 0) { length = 0; }
    var startPos = Math.max(0, newText.lastIndexOf("\n", pos) - 100);
    var endPos = Math.min(newText.length, newText.indexOf("\n", pos + length));
    endPos = endPos === -1 ? newText.length : Math.min(newText.length, endPos + 100);
    var text = "";
    text += newText.substring(startPos, endPos);
    if (startPos !== 0)
        text = "..." + text;
    if (endPos !== newText.length)
        text += "...";
    return text;
}
exports.getTextForError = getTextForError;
