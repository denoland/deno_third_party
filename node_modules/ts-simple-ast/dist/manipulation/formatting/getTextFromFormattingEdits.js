"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
function getTextFromFormattingEdits(sourceFile, formattingEdits) {
    var e_1, _a;
    // reverse the order
    formattingEdits = tslib_1.__spread(formattingEdits).sort(function (a, b) { return b.getSpan().getStart() - a.getSpan().getStart(); });
    var text = sourceFile.getFullText();
    try {
        for (var formattingEdits_1 = tslib_1.__values(formattingEdits), formattingEdits_1_1 = formattingEdits_1.next(); !formattingEdits_1_1.done; formattingEdits_1_1 = formattingEdits_1.next()) {
            var textChange = formattingEdits_1_1.value;
            var span = textChange.getSpan();
            text = text.slice(0, span.getStart()) + textChange.getNewText() + text.slice(span.getEnd());
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (formattingEdits_1_1 && !formattingEdits_1_1.done && (_a = formattingEdits_1.return)) _a.call(formattingEdits_1);
        }
        finally { if (e_1) throw e_1.error; }
    }
    return text;
}
exports.getTextFromFormattingEdits = getTextFromFormattingEdits;
