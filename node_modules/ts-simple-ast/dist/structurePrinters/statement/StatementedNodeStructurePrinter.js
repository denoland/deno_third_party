"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var StatementedNodeStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(StatementedNodeStructurePrinter, _super);
    function StatementedNodeStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        return _this;
    }
    StatementedNodeStructurePrinter.prototype.printText = function (writer, structure) {
        conditionalBlankLine(structure.typeAliases);
        this.factory.forTypeAliasDeclaration().printTexts(writer, structure.typeAliases);
        conditionalBlankLine(structure.interfaces);
        this.factory.forInterfaceDeclaration().printTexts(writer, structure.interfaces);
        conditionalBlankLine(structure.enums);
        this.factory.forEnumDeclaration().printTexts(writer, structure.enums);
        conditionalBlankLine(structure.functions);
        this.factory.forFunctionDeclaration().printTexts(writer, structure.functions);
        conditionalBlankLine(structure.classes);
        this.factory.forClassDeclaration(this.options).printTexts(writer, structure.classes);
        conditionalBlankLine(structure.namespaces);
        this.factory.forNamespaceDeclaration(this.options).printTexts(writer, structure.namespaces);
        function conditionalBlankLine(structures) {
            if (!writer.isAtStartOfFirstLineOfBlock() && !utils_1.ArrayUtils.isNullOrEmpty(structures))
                writer.blankLine();
        }
    };
    return StatementedNodeStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.StatementedNodeStructurePrinter = StatementedNodeStructurePrinter;
