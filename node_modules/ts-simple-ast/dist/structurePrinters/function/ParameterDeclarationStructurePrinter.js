"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var ParameterDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ParameterDeclarationStructurePrinter, _super);
    function ParameterDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.CommaSeparatedStructuresPrinter(_this);
        return _this;
    }
    ParameterDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        if (structures == null || structures.length === 0)
            return;
        this.multipleWriter.printText(writer, structures);
    };
    ParameterDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forDecorator().printTextsInline(writer, structure.decorators);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.conditionalWrite(structure.isRestParameter, "...");
        writer.write(structure.name);
        writer.conditionalWrite(structure.hasQuestionToken, "?");
        this.factory.forTypedNode(":", structure.hasQuestionToken).printText(writer, structure);
        this.factory.forInitializerExpressionableNode().printText(writer, structure);
    };
    return ParameterDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ParameterDeclarationStructurePrinter = ParameterDeclarationStructurePrinter;
