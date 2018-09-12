"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var textSeek_1 = require("../textSeek");
var getTextForError_1 = require("./getTextForError");
var RemoveChildrenTextManipulator = /** @class */ (function () {
    function RemoveChildrenTextManipulator(opts) {
        this.opts = opts;
    }
    RemoveChildrenTextManipulator.prototype.getNewText = function (inputText) {
        var _a = this.opts, children = _a.children, _b = _a.removePrecedingSpaces, removePrecedingSpaces = _b === void 0 ? false : _b, _c = _a.removeFollowingSpaces, removeFollowingSpaces = _c === void 0 ? false : _c, _d = _a.removePrecedingNewLines, removePrecedingNewLines = _d === void 0 ? false : _d, _e = _a.removeFollowingNewLines, removeFollowingNewLines = _e === void 0 ? false : _e;
        var sourceFile = children[0].getSourceFile();
        var fullText = sourceFile.getFullText();
        var removalPos = getRemovalPos();
        this.removalPos = removalPos;
        return getPrefix() + getSuffix();
        function getPrefix() {
            return fullText.substring(0, removalPos);
        }
        function getSuffix() {
            return fullText.substring(getRemovalEnd());
        }
        function getRemovalPos() {
            var pos = children[0].getNonWhitespaceStart();
            if (removePrecedingSpaces || removePrecedingNewLines)
                return textSeek_1.getPreviousMatchingPos(fullText, pos, getCharRemovalFunction(removePrecedingSpaces, removePrecedingNewLines));
            return pos;
        }
        function getRemovalEnd() {
            var end = children[children.length - 1].getEnd();
            if (removeFollowingSpaces || removeFollowingNewLines)
                return textSeek_1.getNextMatchingPos(fullText, end, getCharRemovalFunction(removeFollowingSpaces, removeFollowingNewLines));
            return end;
        }
        function getCharRemovalFunction(removeSpaces, removeNewLines) {
            return function (char) {
                if (removeNewLines && (char === "\r" || char === "\n"))
                    return false;
                if (removeSpaces && !charNotSpaceOrTab(char))
                    return false;
                return true;
            };
        }
        function charNotSpaceOrTab(char) {
            return char !== " " && char !== "\t";
        }
    };
    RemoveChildrenTextManipulator.prototype.getTextForError = function (newText) {
        return getTextForError_1.getTextForError(newText, this.removalPos);
    };
    return RemoveChildrenTextManipulator;
}());
exports.RemoveChildrenTextManipulator = RemoveChildrenTextManipulator;
