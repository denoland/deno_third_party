"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var EnumDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(EnumDeclarationStructurePrinter, _super);
    function EnumDeclarationStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.BlankLineFormattingStructuresPrinter(_this);
        return _this;
    }
    EnumDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    EnumDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var _this = this;
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.conditionalWrite(structure.isConst, "const ");
        writer.write("enum " + structure.name + " ").inlineBlock(function () {
            _this.factory.forEnumMember().printTexts(writer, structure.members);
        });
    };
    return EnumDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.EnumDeclarationStructurePrinter = EnumDeclarationStructurePrinter;
