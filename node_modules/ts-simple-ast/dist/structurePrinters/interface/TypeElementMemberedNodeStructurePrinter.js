"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var TypeElementMemberedNodeStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(TypeElementMemberedNodeStructurePrinter, _super);
    function TypeElementMemberedNodeStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    TypeElementMemberedNodeStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forCallSignatureDeclaration().printTexts(writer, structure.callSignatures);
        this.conditionalSeparator(writer, structure.constructSignatures);
        this.factory.forConstructSignatureDeclaration().printTexts(writer, structure.constructSignatures);
        this.conditionalSeparator(writer, structure.indexSignatures);
        this.factory.forIndexSignatureDeclaration().printTexts(writer, structure.indexSignatures);
        this.conditionalSeparator(writer, structure.properties);
        this.factory.forPropertySignature().printTexts(writer, structure.properties);
        this.conditionalSeparator(writer, structure.methods);
        this.factory.forMethodSignature().printTexts(writer, structure.methods);
    };
    TypeElementMemberedNodeStructurePrinter.prototype.conditionalSeparator = function (writer, structures) {
        if (!utils_1.ArrayUtils.isNullOrEmpty(structures) && !writer.isAtStartOfFirstLineOfBlock())
            writer.newLine();
    };
    return TypeElementMemberedNodeStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.TypeElementMemberedNodeStructurePrinter = TypeElementMemberedNodeStructurePrinter;
