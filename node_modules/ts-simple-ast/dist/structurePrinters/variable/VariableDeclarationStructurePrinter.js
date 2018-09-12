"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var VariableDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(VariableDeclarationStructurePrinter, _super);
    function VariableDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.CommaSeparatedStructuresPrinter(_this);
        return _this;
    }
    VariableDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    VariableDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write(structure.name);
        writer.conditionalWrite(structure.hasExclamationToken, "!");
        this.factory.forTypedNode(":").printText(writer, structure);
        this.factory.forInitializerExpressionableNode().printText(writer, structure);
    };
    return VariableDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.VariableDeclarationStructurePrinter = VariableDeclarationStructurePrinter;
