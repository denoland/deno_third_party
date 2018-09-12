"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getPosAtNextNonBlankLine(text, pos) {
    var newPos = pos;
    for (var i = pos; i < text.length; i++) {
        if (text[i] === " " || text[i] === "\t")
            continue;
        if (text[i] === "\r" && text[i + 1] === "\n" || text[i] === "\n") {
            newPos = i + 1;
            if (text[i] === "\r") {
                i++;
                newPos++;
            }
            continue;
        }
        return newPos;
    }
    return newPos;
}
exports.getPosAtNextNonBlankLine = getPosAtNextNonBlankLine;
