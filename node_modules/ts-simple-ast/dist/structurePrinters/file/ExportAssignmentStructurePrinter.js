"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var ExportAssignmentStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ExportAssignmentStructurePrinter, _super);
    function ExportAssignmentStructurePrinter() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.multipleWriter = new formatting_1.NewLineFormattingStructuresPrinter(_this);
        return _this;
    }
    ExportAssignmentStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    ExportAssignmentStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write("export");
        if (structure.isExportEquals !== false)
            writer.write(" = ");
        else
            writer.write(" default ");
        this.printTextOrWriterFunc(writer, structure.expression);
        writer.write(";");
    };
    return ExportAssignmentStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ExportAssignmentStructurePrinter = ExportAssignmentStructurePrinter;
