"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var RenameLocationTextManipulator = /** @class */ (function () {
    function RenameLocationTextManipulator(renameLocations, newName) {
        this.renameLocations = renameLocations;
        this.newName = newName;
    }
    RenameLocationTextManipulator.prototype.getNewText = function (inputText) {
        var e_1, _a;
        // todo: go in reverse so that the difference doesn't need to be kept track of
        var newFileText = inputText;
        var difference = 0;
        try {
            for (var _b = tslib_1.__values(this.renameLocations.map(function (l) { return l.getTextSpan(); })), _c = _b.next(); !_c.done; _c = _b.next()) {
                var textSpan = _c.value;
                var start = textSpan.getStart();
                var end = start + textSpan.getLength();
                start -= difference;
                end -= difference;
                newFileText = newFileText.substring(0, start) + this.newName + newFileText.substring(end);
                difference += textSpan.getLength() - this.newName.length;
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
        return newFileText;
    };
    RenameLocationTextManipulator.prototype.getTextForError = function (newText) {
        if (this.renameLocations.length === 0)
            return newText;
        return "..." + newText.substring(this.renameLocations[0].getTextSpan().getStart());
    };
    return RenameLocationTextManipulator;
}());
exports.RenameLocationTextManipulator = RenameLocationTextManipulator;
