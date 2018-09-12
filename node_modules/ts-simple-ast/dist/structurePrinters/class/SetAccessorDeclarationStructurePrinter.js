"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var SetAccessorDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(SetAccessorDeclarationStructurePrinter, _super);
    function SetAccessorDeclarationStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        _this.multipleWriter = new formatting_1.BlankLineFormattingStructuresPrinter(_this);
        return _this;
    }
    SetAccessorDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    SetAccessorDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var _this = this;
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forDecorator().printTexts(writer, structure.decorators);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write("set " + structure.name);
        this.factory.forTypeParameterDeclaration().printTextsWithBrackets(writer, structure.typeParameters);
        writer.write("(");
        this.factory.forParameterDeclaration().printTexts(writer, structure.parameters);
        writer.write(")");
        this.factory.forReturnTypedNode().printText(writer, structure);
        writer.spaceIfLastNot().inlineBlock(function () {
            _this.factory.forBodyText(_this.options).printText(writer, structure);
        });
    };
    return SetAccessorDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.SetAccessorDeclarationStructurePrinter = SetAccessorDeclarationStructurePrinter;
