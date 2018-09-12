"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var SourceFileStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(SourceFileStructurePrinter, _super);
    function SourceFileStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        return _this;
    }
    SourceFileStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forImportDeclaration().printTexts(writer, structure.imports);
        this.factory.forBodyText(this.options).printText(writer, structure);
        this.conditionalBlankLine(writer, structure.exports);
        this.factory.forExportDeclaration().printTexts(writer, structure.exports);
        writer.conditionalNewLine(!writer.isAtStartOfFirstLineOfBlock() && !writer.isLastNewLine());
    };
    SourceFileStructurePrinter.prototype.conditionalBlankLine = function (writer, structures) {
        if (!utils_1.ArrayUtils.isNullOrEmpty(structures))
            writer.conditionalBlankLine(!writer.isAtStartOfFirstLineOfBlock());
    };
    return SourceFileStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.SourceFileStructurePrinter = SourceFileStructurePrinter;
