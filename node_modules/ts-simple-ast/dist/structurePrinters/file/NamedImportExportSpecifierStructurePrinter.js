"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var NamedImportExportSpecifierStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(NamedImportExportSpecifierStructurePrinter, _super);
    function NamedImportExportSpecifierStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.CommaSeparatedStructuresPrinter(_this);
        return _this;
    }
    NamedImportExportSpecifierStructurePrinter.prototype.printTextsWithBraces = function (writer, structures) {
        var formatSettings = this.factory.getFormatCodeSettings();
        writer.write("{").conditionalWrite(formatSettings.insertSpaceAfterOpeningAndBeforeClosingNonemptyBraces, " ");
        this.printTexts(writer, structures);
        writer.conditionalWrite(formatSettings.insertSpaceAfterOpeningAndBeforeClosingNonemptyBraces, " ").write("}");
    };
    NamedImportExportSpecifierStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    NamedImportExportSpecifierStructurePrinter.prototype.printText = function (writer, structure) {
        if (typeof structure === "string")
            writer.write(structure);
        else {
            writer.write(structure.name);
            writer.conditionalWrite(structure.alias != null, " as " + structure.alias);
        }
    };
    return NamedImportExportSpecifierStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.NamedImportExportSpecifierStructurePrinter = NamedImportExportSpecifierStructurePrinter;
