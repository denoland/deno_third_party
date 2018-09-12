"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var compiler_1 = require("../compiler");
var typescript_1 = require("../typescript");
var utils_1 = require("../utils");
var SettingsContainer_1 = require("./SettingsContainer");
/** Kinds of indentation */
var IndentationText;
(function (IndentationText) {
    /** Two spaces */
    IndentationText["TwoSpaces"] = "  ";
    /** Four spaces */
    IndentationText["FourSpaces"] = "    ";
    /** Eight spaces */
    IndentationText["EightSpaces"] = "        ";
    /** Tab */
    IndentationText["Tab"] = "\t";
})(IndentationText = exports.IndentationText || (exports.IndentationText = {}));
/**
 * Holds the manipulation settings.
 */
var ManipulationSettingsContainer = /** @class */ (function (_super) {
    tslib_1.__extends(ManipulationSettingsContainer, _super);
    function ManipulationSettingsContainer() {
        return _super.call(this, {
            indentationText: IndentationText.FourSpaces,
            newLineKind: typescript_1.NewLineKind.LineFeed,
            quoteKind: compiler_1.QuoteKind.Double,
            insertSpaceAfterOpeningAndBeforeClosingNonemptyBraces: true
        }) || this;
    }
    /**
     * Gets the editor settings based on the current manipulation settings.
     */
    ManipulationSettingsContainer.prototype.getEditorSettings = function () {
        if (this.editorSettings == null) {
            this.editorSettings = {};
            utils_1.fillDefaultEditorSettings(this.editorSettings, this);
        }
        return tslib_1.__assign({}, this.editorSettings);
    };
    /**
     * Gets the format code settings.
     */
    ManipulationSettingsContainer.prototype.getFormatCodeSettings = function () {
        if (this.formatCodeSettings == null) {
            this.formatCodeSettings = tslib_1.__assign({}, this.getEditorSettings(), { insertSpaceAfterOpeningAndBeforeClosingNonemptyBraces: this.settings.insertSpaceAfterOpeningAndBeforeClosingNonemptyBraces });
        }
        return tslib_1.__assign({}, this.formatCodeSettings);
    };
    /**
     * Gets the user preferences.
     */
    ManipulationSettingsContainer.prototype.getUserPreferences = function () {
        if (this.userPreferences == null) {
            this.userPreferences = {
                quotePreference: this.getQuoteKind() === compiler_1.QuoteKind.Double ? "double" : "single"
            };
        }
        return tslib_1.__assign({}, this.userPreferences);
    };
    /**
     * Gets the quote kind used for string literals.
     */
    ManipulationSettingsContainer.prototype.getQuoteKind = function () {
        return this.settings.quoteKind;
    };
    /**
     * Gets the new line kind.
     */
    ManipulationSettingsContainer.prototype.getNewLineKind = function () {
        return this.settings.newLineKind;
    };
    /**
     * Gets the new line kind as a string.
     */
    ManipulationSettingsContainer.prototype.getNewLineKindAsString = function () {
        return utils_1.newLineKindToString(this.getNewLineKind());
    };
    /**
     * Gets the indentation text;
     */
    ManipulationSettingsContainer.prototype.getIndentationText = function () {
        return this.settings.indentationText;
    };
    /**
     * Sets one or all of the settings.
     * @param settings - Settings to set.
     */
    ManipulationSettingsContainer.prototype.set = function (settings) {
        _super.prototype.set.call(this, settings);
        this.editorSettings = undefined;
        this.formatCodeSettings = undefined;
        this.userPreferences = undefined;
    };
    return ManipulationSettingsContainer;
}(SettingsContainer_1.SettingsContainer));
exports.ManipulationSettingsContainer = ManipulationSettingsContainer;
