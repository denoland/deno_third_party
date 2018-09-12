"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getPosAtEndOfPreviousLine(fullText, pos) {
    while (pos > 0) {
        pos--;
        if (fullText[pos] === "\n") {
            if (fullText[pos - 1] === "\r")
                return pos - 1;
            return pos;
        }
    }
    return pos;
}
exports.getPosAtEndOfPreviousLine = getPosAtEndOfPreviousLine;
