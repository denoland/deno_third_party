"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var errors = require("../errors");
var typescript_1 = require("../typescript");
function newLineKindToString(kind) {
    switch (kind) {
        case typescript_1.NewLineKind.CarriageReturnLineFeed:
            return "\r\n";
        case typescript_1.NewLineKind.LineFeed:
            return "\n";
        default:
            throw new errors.NotImplementedError("Not implemented newline kind: " + kind);
    }
}
exports.newLineKindToString = newLineKindToString;
