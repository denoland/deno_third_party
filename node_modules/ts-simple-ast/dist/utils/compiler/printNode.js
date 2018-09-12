"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var typescript_1 = require("../../typescript");
function printNode(node, sourceFileOrOptions, secondOverloadOptions) {
    var isFirstOverload = sourceFileOrOptions == null || sourceFileOrOptions.kind !== typescript_1.SyntaxKind.SourceFile;
    var options = getOptions();
    var sourceFile = getSourceFile();
    var printer = typescript_1.ts.createPrinter({
        newLine: options.newLineKind == null ? typescript_1.NewLineKind.LineFeed : options.newLineKind,
        removeComments: options.removeComments || false
    });
    if (sourceFile == null)
        return printer.printFile(node);
    else
        return printer.printNode(options.emitHint == null ? typescript_1.EmitHint.Unspecified : options.emitHint, node, sourceFile);
    function getSourceFile() {
        if (isFirstOverload) {
            if (node.kind === typescript_1.SyntaxKind.SourceFile)
                return undefined;
            var scriptKind = getScriptKind();
            return typescript_1.ts.createSourceFile("print." + getFileExt(scriptKind), "", typescript_1.ScriptTarget.Latest, false, scriptKind);
        }
        return sourceFileOrOptions;
        function getScriptKind() {
            return options.scriptKind == null ? typescript_1.ScriptKind.TSX : options.scriptKind;
        }
        function getFileExt(scriptKind) {
            if (scriptKind === typescript_1.ScriptKind.JSX || scriptKind === typescript_1.ScriptKind.TSX)
                return "tsx";
            return "ts";
        }
    }
    function getOptions() {
        return (isFirstOverload ? sourceFileOrOptions : secondOverloadOptions) || {};
    }
}
exports.printNode = printNode;
