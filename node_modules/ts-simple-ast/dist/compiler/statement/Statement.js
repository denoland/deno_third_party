"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var common_1 = require("../common");
var Statement = /** @class */ (function (_super) {
    tslib_1.__extends(Statement, _super);
    function Statement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Removes the statement.
     */
    Statement.prototype.remove = function () {
        manipulation_1.removeStatementedNodeChild(this);
    };
    return Statement;
}(common_1.Node));
exports.Statement = Statement;
