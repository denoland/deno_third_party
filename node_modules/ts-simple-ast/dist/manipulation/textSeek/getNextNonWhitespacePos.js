"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var getNextMatchingPos_1 = require("./getNextMatchingPos");
function getNextNonWhitespacePos(text, pos) {
    return getNextMatchingPos_1.getNextMatchingPos(text, pos, function (char) { return char !== " " && char !== "\t" && char !== "\r" && char !== "\n"; });
}
exports.getNextNonWhitespacePos = getNextNonWhitespacePos;
