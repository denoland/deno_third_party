"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var options_1 = require("../options");
var typescript_1 = require("../typescript");
var setValueIfUndefined_1 = require("./setValueIfUndefined");
function fillDefaultEditorSettings(settings, manipulationSettings) {
    setValueIfUndefined_1.setValueIfUndefined(settings, "convertTabsToSpaces", manipulationSettings.getIndentationText() !== options_1.IndentationText.Tab);
    setValueIfUndefined_1.setValueIfUndefined(settings, "newLineCharacter", manipulationSettings.getNewLineKindAsString());
    setValueIfUndefined_1.setValueIfUndefined(settings, "indentStyle", typescript_1.IndentStyle.Smart);
    setValueIfUndefined_1.setValueIfUndefined(settings, "indentSize", manipulationSettings.getIndentationText().length);
    setValueIfUndefined_1.setValueIfUndefined(settings, "tabSize", manipulationSettings.getIndentationText().length);
}
exports.fillDefaultEditorSettings = fillDefaultEditorSettings;
