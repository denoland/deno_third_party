"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var TypeParameterDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(TypeParameterDeclarationStructurePrinter, _super);
    function TypeParameterDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.CommaSeparatedStructuresPrinter(_this);
        return _this;
    }
    TypeParameterDeclarationStructurePrinter.prototype.printTextsWithBrackets = function (writer, structures) {
        if (structures == null || structures.length === 0)
            return;
        writer.write("<");
        this.printTexts(writer, structures);
        writer.write(">");
    };
    TypeParameterDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    TypeParameterDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write(structure.name);
        if (structure.constraint != null && structure.constraint.length > 0)
            writer.write(" extends " + structure.constraint);
        if (structure.default != null && structure.default.length > 0)
            writer.write(" = " + structure.default);
    };
    return TypeParameterDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.TypeParameterDeclarationStructurePrinter = TypeParameterDeclarationStructurePrinter;
