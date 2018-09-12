"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var fillDefaultEditorSettings_1 = require("./fillDefaultEditorSettings");
var setValueIfUndefined_1 = require("./setValueIfUndefined");
function fillDefaultFormatCodeSettings(settings, manipulationSettings) {
    fillDefaultEditorSettings_1.fillDefaultEditorSettings(settings, manipulationSettings);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterCommaDelimiter", true);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterConstructor", false);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterSemicolonInForStatements", true);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterKeywordsInControlFlowStatements", true);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterOpeningAndBeforeClosingNonemptyBraces", true);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterOpeningAndBeforeClosingNonemptyBrackets", false);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterOpeningAndBeforeClosingTemplateStringBraces", false);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceAfterOpeningAndBeforeClosingJsxExpressionBraces", false);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceBeforeFunctionParenthesis", false);
    setValueIfUndefined_1.setValueIfUndefined(settings, "insertSpaceBeforeAndAfterBinaryOperators", true);
    setValueIfUndefined_1.setValueIfUndefined(settings, "placeOpenBraceOnNewLineForFunctions", false);
    setValueIfUndefined_1.setValueIfUndefined(settings, "placeOpenBraceOnNewLineForControlBlocks", false);
    setValueIfUndefined_1.setValueIfUndefined(settings, "ensureNewLineAtEndOfFile", true);
}
exports.fillDefaultFormatCodeSettings = fillDefaultFormatCodeSettings;
