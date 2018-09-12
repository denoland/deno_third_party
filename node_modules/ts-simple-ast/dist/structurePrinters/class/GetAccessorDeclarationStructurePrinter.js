"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var GetAccessorDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(GetAccessorDeclarationStructurePrinter, _super);
    function GetAccessorDeclarationStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        _this.blankLineWriter = new formatting_1.BlankLineFormattingStructuresPrinter(_this);
        return _this;
    }
    GetAccessorDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.blankLineWriter.printText(writer, structures);
    };
    GetAccessorDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var _this = this;
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forDecorator().printTexts(writer, structure.decorators);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write("get " + structure.name);
        this.factory.forTypeParameterDeclaration().printTextsWithBrackets(writer, structure.typeParameters);
        writer.write("(");
        this.factory.forParameterDeclaration().printTexts(writer, structure.parameters);
        writer.write(")");
        this.factory.forReturnTypedNode().printText(writer, structure);
        writer.spaceIfLastNot().inlineBlock(function () {
            _this.factory.forBodyText(_this.options).printText(writer, structure);
        });
    };
    return GetAccessorDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.GetAccessorDeclarationStructurePrinter = GetAccessorDeclarationStructurePrinter;
