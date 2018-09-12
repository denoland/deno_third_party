"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var CallSignatureDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(CallSignatureDeclarationStructurePrinter, _super);
    function CallSignatureDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    CallSignatureDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    CallSignatureDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forTypeParameterDeclaration().printTextsWithBrackets(writer, structure.typeParameters);
        writer.write("(");
        this.factory.forParameterDeclaration().printTexts(writer, structure.parameters);
        writer.write(")");
        this.factory.forReturnTypedNode(true).printText(writer, structure);
        writer.write(";");
    };
    return CallSignatureDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.CallSignatureDeclarationStructurePrinter = CallSignatureDeclarationStructurePrinter;
