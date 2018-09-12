"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var ExportDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ExportDeclarationStructurePrinter, _super);
    function ExportDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    ExportDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    ExportDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var hasModuleSpecifier = structure.moduleSpecifier != null && structure.moduleSpecifier.length > 0;
        writer.write("export");
        if (structure.namedExports != null && structure.namedExports.length > 0) {
            writer.space();
            this.factory.forNamedImportExportSpecifier().printTextsWithBraces(writer, structure.namedExports);
        }
        else if (!hasModuleSpecifier)
            writer.write(" {")
                .conditionalWrite(this.factory.getFormatCodeSettings().insertSpaceAfterOpeningAndBeforeClosingNonemptyBraces, " ") // compiler does this
                .write("}");
        else
            writer.write(" *");
        if (hasModuleSpecifier) {
            writer.write(" from ");
            writer.quote(structure.moduleSpecifier);
        }
        writer.write(";");
    };
    return ExportDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ExportDeclarationStructurePrinter = ExportDeclarationStructurePrinter;
