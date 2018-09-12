"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var PropertyDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(PropertyDeclarationStructurePrinter, _super);
    function PropertyDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    PropertyDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    PropertyDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forDecorator().printTexts(writer, structure.decorators);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write(structure.name);
        writer.conditionalWrite(structure.hasQuestionToken, "?");
        writer.conditionalWrite(structure.hasExclamationToken && !structure.hasQuestionToken, "!");
        this.factory.forTypedNode(":").printText(writer, structure);
        this.factory.forInitializerExpressionableNode().printText(writer, structure);
        writer.write(";");
    };
    return PropertyDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.PropertyDeclarationStructurePrinter = PropertyDeclarationStructurePrinter;
