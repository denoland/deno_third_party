"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var NamespaceDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(NamespaceDeclarationStructurePrinter, _super);
    function NamespaceDeclarationStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        _this.blankLineFormattingWriter = new formatting_1.BlankLineFormattingStructuresPrinter(_this);
        return _this;
    }
    NamespaceDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.blankLineFormattingWriter.printText(writer, structures);
    };
    NamespaceDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var _this = this;
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write((structure.hasModuleKeyword ? "module" : "namespace") + " " + structure.name + " ").inlineBlock(function () {
            _this.factory.forBodyText({
                isAmbient: structure.hasDeclareKeyword || _this.options.isAmbient
            }).printText(writer, structure);
        });
    };
    return NamespaceDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.NamespaceDeclarationStructurePrinter = NamespaceDeclarationStructurePrinter;
