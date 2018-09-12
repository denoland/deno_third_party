"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var IndexSignatureDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(IndexSignatureDeclarationStructurePrinter, _super);
    function IndexSignatureDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    IndexSignatureDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    IndexSignatureDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write("[" + (structure.keyName || "key") + ": " + (structure.keyType || "string") + "]: ");
        utils_1.printTextFromStringOrWriter(writer, structure.returnType);
        writer.write(";");
    };
    return IndexSignatureDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.IndexSignatureDeclarationStructurePrinter = IndexSignatureDeclarationStructurePrinter;
