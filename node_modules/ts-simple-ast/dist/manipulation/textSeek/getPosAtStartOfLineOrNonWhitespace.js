"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getPosAtStartOfLineOrNonWhitespace(fullText, pos) {
    while (pos > 0) {
        pos--;
        var currentChar = fullText[pos];
        if (currentChar === "\n")
            return pos + 1;
        else if (currentChar !== " " && currentChar !== "\t")
            return pos + 1;
    }
    return pos;
}
exports.getPosAtStartOfLineOrNonWhitespace = getPosAtStartOfLineOrNonWhitespace;
