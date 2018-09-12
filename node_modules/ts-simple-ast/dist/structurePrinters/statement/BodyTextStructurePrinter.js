"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var codeBlockWriter_1 = require("../../codeBlockWriter");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var BodyTextStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(BodyTextStructurePrinter, _super);
    function BodyTextStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        return _this;
    }
    BodyTextStructurePrinter.prototype.printText = function (writer, structure) {
        this.factory.forStatementedNode(this.options).printText(writer, structure);
        // todo: hacky, will need to change this in the future...
        // basically, need a way to make this only do the blank line if the user does a write
        var newWriter = new codeBlockWriter_1.CodeBlockWriter(writer.getOptions());
        this.printTextOrWriterFunc(newWriter, structure.bodyText);
        if (newWriter.getLength() > 0) {
            if (!writer.isAtStartOfFirstLineOfBlock())
                writer.blankLineIfLastNot();
            writer.write(newWriter.toString());
        }
    };
    return BodyTextStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.BodyTextStructurePrinter = BodyTextStructurePrinter;
