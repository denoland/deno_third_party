"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var InsertionTextManipulator_1 = require("./InsertionTextManipulator");
var UnwrapTextManipulator = /** @class */ (function (_super) {
    tslib_1.__extends(UnwrapTextManipulator, _super);
    function UnwrapTextManipulator(node) {
        return _super.call(this, {
            insertPos: node.getPos(),
            newText: getReplacementText(node),
            replacingLength: node.getFullWidth()
        }) || this;
    }
    return UnwrapTextManipulator;
}(InsertionTextManipulator_1.InsertionTextManipulator));
exports.UnwrapTextManipulator = UnwrapTextManipulator;
function getReplacementText(node) {
    var e_1, _a;
    var childSyntaxList = node.getChildSyntaxListOrThrow();
    var indentationText = node.getIndentationText();
    var childIndentationText = node.getChildIndentationText();
    var indentationDifference = childIndentationText.replace(indentationText, "");
    var replaceRegex = new RegExp("^" + indentationDifference);
    var originalText = childSyntaxList.getFullText();
    var sourceFile = node.sourceFile;
    var lines = originalText.split("\n");
    var pos = childSyntaxList.getPos();
    var newLines = [];
    try {
        for (var lines_1 = tslib_1.__values(lines), lines_1_1 = lines_1.next(); !lines_1_1.done; lines_1_1 = lines_1.next()) {
            var line = lines_1_1.value;
            if (sourceFile.isInStringAtPos(pos))
                newLines.push(line);
            else
                newLines.push(line.replace(replaceRegex, ""));
            pos += line.length + 1;
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (lines_1_1 && !lines_1_1.done && (_a = lines_1.return)) _a.call(lines_1);
        }
        finally { if (e_1) throw e_1.error; }
    }
    return newLines.join("\n").replace(/^\r?\n/, "");
}
