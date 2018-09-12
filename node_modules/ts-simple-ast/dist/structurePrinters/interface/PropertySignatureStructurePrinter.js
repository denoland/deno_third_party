"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var PropertySignatureStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(PropertySignatureStructurePrinter, _super);
    function PropertySignatureStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    PropertySignatureStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    PropertySignatureStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write(structure.name);
        writer.conditionalWrite(structure.hasQuestionToken, "?");
        this.factory.forTypedNode(":").printText(writer, structure);
        // why would someone write an initializer? I guess let them do it...
        this.factory.forInitializerExpressionableNode().printText(writer, structure);
        writer.write(";");
    };
    return PropertySignatureStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.PropertySignatureStructurePrinter = PropertySignatureStructurePrinter;
