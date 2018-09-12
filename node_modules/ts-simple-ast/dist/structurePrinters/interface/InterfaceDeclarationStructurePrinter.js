"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var InterfaceDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(InterfaceDeclarationStructurePrinter, _super);
    function InterfaceDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.BlankLineFormattingStructuresPrinter(_this);
        return _this;
    }
    InterfaceDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    InterfaceDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var _this = this;
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write("interface " + structure.name);
        this.factory.forTypeParameterDeclaration().printTextsWithBrackets(writer, structure.typeParameters);
        writer.space();
        if (!utils_1.ArrayUtils.isNullOrEmpty(structure.extends))
            writer.write("extends " + structure.extends.join(", ") + " ");
        writer.inlineBlock(function () {
            _this.factory.forTypeElementMemberedNode().printText(writer, structure);
        });
    };
    return InterfaceDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.InterfaceDeclarationStructurePrinter = InterfaceDeclarationStructurePrinter;
