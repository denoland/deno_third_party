"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var getTextForError_1 = require("./getTextForError");
var InsertionTextManipulator = /** @class */ (function () {
    function InsertionTextManipulator(opts) {
        this.opts = opts;
    }
    InsertionTextManipulator.prototype.getNewText = function (inputText) {
        var _a = this.opts, insertPos = _a.insertPos, newText = _a.newText, _b = _a.replacingLength, replacingLength = _b === void 0 ? 0 : _b;
        return inputText.substring(0, insertPos) + newText + inputText.substring(insertPos + replacingLength);
    };
    InsertionTextManipulator.prototype.getTextForError = function (newText) {
        return getTextForError_1.getTextForError(newText, this.opts.insertPos, this.opts.newText.length);
    };
    return InsertionTextManipulator;
}());
exports.InsertionTextManipulator = InsertionTextManipulator;
