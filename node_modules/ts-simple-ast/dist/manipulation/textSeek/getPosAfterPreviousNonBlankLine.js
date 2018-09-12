"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function getPosAfterPreviousNonBlankLine(text, pos) {
    var newPos = pos;
    for (var i = pos - 1; i >= 0; i--) {
        if (text[i] === " " || text[i] === "\t")
            continue;
        if (text[i] === "\n") {
            newPos = i + 1;
            if (text[i - 1] === "\r")
                i--;
            continue;
        }
        return newPos;
    }
    return 0;
}
exports.getPosAfterPreviousNonBlankLine = getPosAfterPreviousNonBlankLine;
