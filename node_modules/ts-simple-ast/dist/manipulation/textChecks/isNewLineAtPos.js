"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function isNewLineAtPos(fullText, pos) {
    return fullText[pos] === "\n" || (fullText[pos] === "\r" && fullText[pos + 1] === "\n");
}
exports.isNewLineAtPos = isNewLineAtPos;
