"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
// todo: tests
function getPreviousMatchingPos(text, pos, condition) {
    while (pos > 0) {
        var char = text[pos - 1];
        if (!condition(char))
            pos--;
        else
            break;
    }
    return pos;
}
exports.getPreviousMatchingPos = getPreviousMatchingPos;
