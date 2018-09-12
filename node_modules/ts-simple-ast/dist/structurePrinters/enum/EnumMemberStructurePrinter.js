"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var EnumMemberStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(EnumMemberStructurePrinter, _super);
    function EnumMemberStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.CommaNewLineSeparatedStructuresPrinter(_this);
        return _this;
    }
    EnumMemberStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    EnumMemberStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        writer.write(structure.name);
        if (typeof structure.value === "string")
            writer.write(" = ").quote(structure.value);
        else if (typeof structure.value === "number")
            writer.write(" = " + structure.value);
        else
            this.factory.forInitializerExpressionableNode().printText(writer, structure);
    };
    return EnumMemberStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.EnumMemberStructurePrinter = EnumMemberStructurePrinter;
