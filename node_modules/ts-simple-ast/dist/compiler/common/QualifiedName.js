"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Node_1 = require("./Node");
var QualifiedName = /** @class */ (function (_super) {
    tslib_1.__extends(QualifiedName, _super);
    function QualifiedName() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the left side of the qualified name.
     */
    QualifiedName.prototype.getLeft = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.left);
    };
    /**
     * Gets the right identifier of the qualified name.
     */
    QualifiedName.prototype.getRight = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.right);
    };
    return QualifiedName;
}(Node_1.Node));
exports.QualifiedName = QualifiedName;
