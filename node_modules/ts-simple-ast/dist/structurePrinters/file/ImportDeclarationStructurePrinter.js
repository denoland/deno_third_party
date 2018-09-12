"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var ImportDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ImportDeclarationStructurePrinter, _super);
    function ImportDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    ImportDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    ImportDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var hasNamedImport = structure.namedImports != null && structure.namedImports.length > 0;
        // validation
        if (hasNamedImport && structure.namespaceImport != null)
            throw new errors.InvalidOperationError("An import declaration cannot have both a namespace import and a named import.");
        writer.write("import");
        // default import
        if (structure.defaultImport != null) {
            writer.write(" " + structure.defaultImport);
            writer.conditionalWrite(hasNamedImport || structure.namespaceImport != null, ",");
        }
        // namespace import
        if (structure.namespaceImport != null)
            writer.write(" * as " + structure.namespaceImport);
        // named imports
        if (structure.namedImports != null && structure.namedImports.length > 0) {
            writer.space();
            this.factory.forNamedImportExportSpecifier().printTextsWithBraces(writer, structure.namedImports);
        }
        // from keyword
        writer.conditionalWrite(structure.defaultImport != null || hasNamedImport || structure.namespaceImport != null, " from");
        writer.write(" ");
        writer.quote(structure.moduleSpecifier);
        writer.write(";");
    };
    return ImportDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ImportDeclarationStructurePrinter = ImportDeclarationStructurePrinter;
