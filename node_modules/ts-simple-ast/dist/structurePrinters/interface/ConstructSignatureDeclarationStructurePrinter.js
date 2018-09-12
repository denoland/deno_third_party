"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var ConstructSignatureDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ConstructSignatureDeclarationStructurePrinter, _super);
    function ConstructSignatureDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    ConstructSignatureDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    ConstructSignatureDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        writer.write("new");
        this.factory.forTypeParameterDeclaration().printTextsWithBrackets(writer, structure.typeParameters);
        writer.write("(");
        this.factory.forParameterDeclaration().printTexts(writer, structure.parameters);
        writer.write(")");
        this.factory.forReturnTypedNode().printText(writer, structure);
        writer.write(";");
    };
    return ConstructSignatureDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ConstructSignatureDeclarationStructurePrinter = ConstructSignatureDeclarationStructurePrinter;
