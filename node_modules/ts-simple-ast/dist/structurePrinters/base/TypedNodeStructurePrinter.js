"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var codeBlockWriter_1 = require("../../codeBlockWriter");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var TypedNodeStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(TypedNodeStructurePrinter, _super);
    function TypedNodeStructurePrinter(factory, separator, alwaysWrite) {
        if (alwaysWrite === void 0) { alwaysWrite = false; }
        var _this = _super.call(this, factory) || this;
        _this.separator = separator;
        _this.alwaysWrite = alwaysWrite;
        return _this;
    }
    TypedNodeStructurePrinter.prototype.printText = function (writer, structure) {
        var type = structure.type;
        if (type == null && this.alwaysWrite === false)
            return;
        type = type || "any";
        // todo: hacky, will need to change this in the future...
        var initializerText = typeof type === "string" ? type : getTextForWriterFunc(type);
        if (!utils_1.StringUtils.isNullOrWhitespace(initializerText))
            writer.write(this.separator + " " + initializerText);
        function getTextForWriterFunc(writerFunc) {
            var newWriter = new codeBlockWriter_1.CodeBlockWriter(writer.getOptions());
            writerFunc(newWriter);
            return newWriter.toString();
        }
    };
    return TypedNodeStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.TypedNodeStructurePrinter = TypedNodeStructurePrinter;
