"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getPosAtEndOfPreviousLineOrNonWhitespace(fullText, pos) {
    while (pos > 0) {
        pos--;
        var currentChar = fullText[pos];
        if (currentChar === "\n") {
            if (fullText[pos - 1] === "\r")
                return pos - 1;
            return pos;
        }
        else if (currentChar !== " " && currentChar !== "\t")
            return pos + 1;
    }
    return pos;
}
exports.getPosAtEndOfPreviousLineOrNonWhitespace = getPosAtEndOfPreviousLineOrNonWhitespace;
