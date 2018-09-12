"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var ModifierableNodeStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ModifierableNodeStructurePrinter, _super);
    function ModifierableNodeStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    ModifierableNodeStructurePrinter.prototype.printText = function (writer, structure) {
        var scope = structure.scope;
        if (structure.isDefaultExport)
            writer.write("export default ");
        else if (structure.isExported)
            writer.write("export ");
        if (structure.hasDeclareKeyword)
            writer.write("declare ");
        if (structure.isAbstract)
            writer.write("abstract ");
        if (scope != null)
            writer.write(scope + " ");
        if (structure.isStatic)
            writer.write("static ");
        if (structure.isAsync)
            writer.write("async ");
        if (structure.isReadonly)
            writer.write("readonly ");
    };
    return ModifierableNodeStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ModifierableNodeStructurePrinter = ModifierableNodeStructurePrinter;
