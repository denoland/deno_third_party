"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var codeBlockWriter_1 = require("../../codeBlockWriter");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var InitializerExpressionableNodeStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(InitializerExpressionableNodeStructurePrinter, _super);
    function InitializerExpressionableNodeStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    InitializerExpressionableNodeStructurePrinter.prototype.printText = function (writer, structure) {
        var initializer = structure.initializer;
        if (initializer == null)
            return;
        // todo: hacky, will need to change this in the future...
        var initializerText = typeof initializer === "string" ? initializer : getTextForWriterFunc(initializer);
        if (!utils_1.StringUtils.isNullOrWhitespace(initializerText))
            writer.write(" = " + initializerText);
        function getTextForWriterFunc(writerFunc) {
            var newWriter = new codeBlockWriter_1.CodeBlockWriter(writer.getOptions());
            writerFunc(newWriter);
            return newWriter.toString();
        }
    };
    return InitializerExpressionableNodeStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.InitializerExpressionableNodeStructurePrinter = InitializerExpressionableNodeStructurePrinter;
